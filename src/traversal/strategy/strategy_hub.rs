use base64::Engine;
use concurrent_queue::ConcurrentQueue;
use dashmap::DashMap;
use itertools::Itertools;
use crate::config::NUM_THREADS;
use crate::traversal::action_history::action::DEFAULT_ACTION_COUNT;
use crate::models::Card;
use crate::traversal::strategy::strategy_hub;
use super::play_strategy::PlayStrategy;
use super::strategy_branch::{StrategyBranch, StrategyHubKey};
use super::strategy_trait::Strategy;
use super::training_strategy::TrainingStrategy;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

pub const MAX_RETRIES: usize = 1000;
#[derive(Default, Debug)]
pub struct StrategyPair<TStrategy: Strategy> {
    pub sb_branch: StrategyBranch<TStrategy>,
    pub bb_branch: StrategyBranch<TStrategy>,
}

// TODO - this needs a total reword. Instead should generate a long list of combinations which we can pop off
#[derive(Debug)]
pub struct StrategyHub<TStrategy: Strategy + Debug> {
    out_queue: ConcurrentQueue<StrategyPair<TStrategy>>,
    pub bb_in_store: DashMap<StrategyHubKey, StrategyBranch<TStrategy>>,
    pub sb_in_store: DashMap<StrategyHubKey, StrategyBranch<TStrategy>>,
}

impl<TStrategy: Strategy + Debug> StrategyHub<TStrategy> {
    pub fn new(strategy_branches: Vec<StrategyBranch<TStrategy>>) -> StrategyHub<TStrategy> {
        let num_elements = strategy_branches.len();
        let bb_in_store = DashMap::new();
        let sb_in_store = DashMap::new();

        for branch in strategy_branches {
            let key = branch.strategy_hub_key.clone();
            match key.is_sb {
                true => {
                    sb_in_store.insert(key, branch);
                },
                false => {
                    bb_in_store.insert(key, branch);
                }
            }
        }
        StrategyHub {
            out_queue: ConcurrentQueue::bounded(num_elements),
            bb_in_store,
            sb_in_store,
        }
    }


    // Grabs a bunch of strategies for which we will iterate over all combinations of BB SB strategies
    // Might be more efficient to do a stream approach but requires profiling to see
    pub fn get_more_elements(&self) -> StrategyPair<TStrategy> {        
        let return_elements = self.out_queue.pop();
        match return_elements {
            Ok(strategy_pair) => {
                strategy_pair
            },
            Err(_) => {
                self.regenerate_queue()
            }
        }
    }

    fn regenerate_queue(&self) -> StrategyPair<TStrategy> {
        for _ in 0..MAX_RETRIES {
            self.generate_queue();
            if let Ok(pair) = self.out_queue.pop() {
                return pair;
            }
        }
        panic!("Failed to regenerate queue after {} attempts", MAX_RETRIES);
    }

    fn generate_queue(&self) {
        let cards = Card::shuffle_deck();

        for i in (0..48).step_by(4) {
            let bb_1 = cards[i];
            let bb_2 = cards[i+1];
            
            let sb_1 = cards[i+2];
            let sb_2 = cards[i+3];
            
            let bb_key = StrategyHubKey {
                low_rank: bb_1.rank.min(bb_2.rank),
                high_rank: bb_1.rank.max(bb_2.rank),
                is_suited: bb_1.suit == bb_2.suit,
                is_sb: false,
            };

            let sb_key = StrategyHubKey {
                low_rank: sb_1.rank.min(sb_2.rank),
                high_rank: sb_1.rank.max(sb_2.rank),
                is_suited: sb_1.suit == sb_2.suit,
                is_sb: true,
            };

            let bb_strategy = self.bb_in_store.remove(&bb_key);
            if bb_strategy.is_some() {
                let sb_strategy = self.sb_in_store.remove(&sb_key);
                if sb_strategy.is_some() {
                    self.out_queue.push(StrategyPair {
                        sb_branch: sb_strategy.unwrap().1,
                        bb_branch: bb_strategy.unwrap().1,
                    }).expect("Should not fail to push to queue");
                } else {
                    self.bb_in_store.insert(bb_key, bb_strategy.unwrap().1);
                }
            }
        }
    }

    pub fn return_strategies(&self, pair: StrategyPair<TStrategy>) {
        match self.bb_in_store.insert(pair.bb_branch.strategy_hub_key.clone(), pair.bb_branch) {
            None => {},
            Some(old_val) => {
                panic!("Duplicate bb strategy {:?}", old_val);
            }
        };
        // self.sb_in_store.insert(pair.sb_branch.strategy_hub_key.clone(), pair.sb_branch).expect("could not reinsert sb strategy, should not happen");
        match self.sb_in_store.insert(pair.sb_branch.strategy_hub_key.clone(), pair.sb_branch) {
            None => {},
            Some(old_val) => {
                panic!("Duplicate sb strategy {:?}", old_val);
            }
        };
    } 

    pub fn into_map(self) -> HashMap<StrategyHubKey, StrategyBranch<TStrategy>> {
        let mut res = HashMap::new();
        while let Ok(pair) = self.out_queue.pop() {
            res.insert(pair.bb_branch.strategy_hub_key.clone(), pair.bb_branch);
            res.insert(pair.sb_branch.strategy_hub_key.clone(), pair.sb_branch);
        }
        let sb_keys = self.sb_in_store.iter().map(|entry| entry.key().clone()).collect::<Vec<_>>();
        let bb_keys = self.bb_in_store.iter().map(|entry| entry.key().clone()).collect::<Vec<_>>();
        for key in sb_keys {
            match self.sb_in_store.remove(&key) {
                Some((_, strategy)) => {
                    res.insert(key, strategy);
                },
                None => {
                    panic!("Failed to remove sb strategy {:?}", key);
                }
            }
        }
        for key in bb_keys {
            match self.bb_in_store.remove(&key) {
                Some((_, strategy)) => {
                    res.insert(key, strategy);
                },
                None => {
                    panic!("Failed to remove bb strategy {:?}", key);
                }
            }
        }
        res
    }

    pub fn from_map(map: HashMap<StrategyHubKey, StrategyBranch<TStrategy>>) -> Result<StrategyHub<TStrategy>, &'static str> {
        let out_queue = ConcurrentQueue::bounded(map.len());
        let sb_in_store = DashMap::new();
        let bb_in_store = DashMap::new();

        for (strategy_key, strategy) in map {
            match strategy.strategy_hub_key.is_sb {
                true => {
                    match sb_in_store.insert(strategy_key.clone(), strategy) {
                        None => {},
                        Some(_) => {
                            panic!("Duplicate sb strategy {:?}", strategy_key);
                        }
                    }
                },
                false => {
                    match bb_in_store.insert(strategy_key.clone(), strategy) {
                        None => {},
                        Some(_) => {
                            panic!("Duplicate bb strategy {:?}", strategy_key);
                        }
                    }
                }
            }
        }

        Result::Ok(StrategyHub {
            out_queue,
            bb_in_store,
            sb_in_store,
        })
    }
}

use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json;
use std::io::{self, Write};
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::Read;

pub fn serialise_strategy_hub(
    output_folder: &str,
    mut strategy_hub: StrategyHub<TrainingStrategy>,
) -> io::Result<()> {
    let strategy_hub = strategy_hub.into_map();
    println!("serialising strategy hub to {}", output_folder);
    // Ensure the output directory exists
    std::fs::create_dir_all(output_folder)?;

    for (strategy_key, strategy_branch) in strategy_hub {
        let path = format!(
            "{}/{}_{}_{}_{}.json.gz",
            output_folder,
            strategy_key.low_rank,
            strategy_key.high_rank,
            if strategy_key.is_suited { "suited" } else { "offsuit" },
            if strategy_key.is_sb { "sb" } else { "bb" }
        );

        strategy_branch.print_stats();

        let serialised = serde_json::to_string(
            &strategy_branch.map.into_iter().map(|(k, v)| {
                (
                    base64::engine::general_purpose::STANDARD.encode(&k),
                    {
                        let mut array = [0.0; DEFAULT_ACTION_COUNT + 1];
                        array[0] = v.actions as f32;
                        array[1..].copy_from_slice(&PlayStrategy::from_train_strategy(v).get_current_strategy(0));
                        array
                    },
                )
            }).collect::<HashMap<_, _>>(),
        )?;

        let compressed_bytes = {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(serialised.as_bytes())?;
            encoder.finish()?
        };

        std::fs::write(&path, &compressed_bytes)?;
    }

    println!("Successfully serialised strategy hub to {}", output_folder);
    Ok(())
}

// Deserialise the strategy hub with multi threading
pub fn deserialise_strategy_hub<TStrategy: Strategy + Debug + Send + Sync + 'static>(blueprint_folder: &str) -> Result<HashMap<StrategyHubKey, StrategyBranch<TStrategy>>, io::Error> {
    fn parse_filename_to_strategy_element(filename: &str) -> Result<StrategyHubKey, io::Error> {
        let parts: Vec<&str> = filename.split('_').collect();
        if parts.len() != 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid filename format"));
        }

        Ok(StrategyHubKey {
            low_rank: parts[0].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid low rank"))?,
            high_rank: parts[1].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid high rank"))?,
            is_suited: parts[2] == "suited",
            is_sb: parts[3] == "sb",
        })
    }

    println!("Deserialising strategy hub from {}", blueprint_folder);

    let strategy_hub_map = Arc::new(DashMap::new());
    let mut total_strategy_size_bytes_compressed = 0;
    let mut total_strategy_size_bytes_uncompressed = 0;

    // if the blueprint folder doesn't exist, create it
    if !std::path::Path::new(blueprint_folder).exists() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Blueprint folder does not exist"));
    }

    let blueprint_files = std::fs::read_dir(blueprint_folder)
        .unwrap()
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect::<Vec<_>>();
    let num_files = blueprint_files.len();
    let chunks = blueprint_files.into_iter().chunks(num_files / NUM_THREADS);
    let blueprint_file_chunks = chunks.into_iter().collect::<Vec<_>>();

    fn deserialisation_work<TStrategy: Strategy + 'static>(
        blueprint_files: Vec<std::path::PathBuf>,
        strategy_hub_map: &Arc<DashMap<StrategyHubKey, StrategyBranch<TStrategy>>>,
    ) -> io::Result<(u64, usize)> { // return the compressed size and uncompressed size for logging
        let mut strategy_size_bytes_compressed = 0;
        let mut strategy_size_bytes_uncompressed = 0;
        for path in blueprint_files {
            if path.extension().and_then(|s| s.to_str()) != Some("gz") {
                continue;
            }

            let mut decompressed_data = String::new();
            GzDecoder::new(File::open(&path)?).read_to_string(&mut decompressed_data)?;

            // The +1 is for the number of actions - these are stored in cell 0
            let deserialised: HashMap<String, [f32; DEFAULT_ACTION_COUNT + 1]> = serde_json::from_str(&decompressed_data)?;

            let strategy_hub_element_key = parse_filename_to_strategy_element({
                    strategy_size_bytes_compressed += path.metadata()?.len();
                    path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid file stem"))?
                    .trim_end_matches(".json")
                }
            )?;

            println!("Deserialising strategy hub element {}", strategy_hub_element_key);

            let map = deserialised.into_iter().map(|(k, v)| {
                let infoset_key = base64::engine::general_purpose::STANDARD.decode(&k).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid base64 key"))?;
                let mut array = [0.0; DEFAULT_ACTION_COUNT];
                array.copy_from_slice(&v[1..]);
                strategy_size_bytes_uncompressed += std::mem::size_of_val(&array) + std::mem::size_of_val(&infoset_key);
                let play_strategy = TStrategy::from_existing_strategy(v[0] as usize, {
                    array
                });
                Ok((infoset_key, play_strategy))
            }).collect::<Result<HashMap<_, _>, io::Error>>()?;

            strategy_hub_map.insert(strategy_hub_element_key.clone(), StrategyBranch {
                strategy_hub_key: strategy_hub_element_key,
                map,
                new_generated: 0,
            });
        }
        Ok((strategy_size_bytes_compressed, strategy_size_bytes_uncompressed))
    }

    let mut handles = Vec::with_capacity(NUM_THREADS);
    
    for chunk in blueprint_file_chunks {
        let chunk = chunk.collect_vec();
        let strategy_hub_map = Arc::clone(&strategy_hub_map);
        handles.push(std::thread::spawn(move || {
            deserialisation_work::<TStrategy>(chunk, &strategy_hub_map)
        }));
    }

    for handle in handles {
        let (strategy_size_bytes_compressed, strategy_size_bytes_uncompressed) = handle.join().unwrap()?;
        total_strategy_size_bytes_compressed += strategy_size_bytes_compressed;
        total_strategy_size_bytes_uncompressed += strategy_size_bytes_uncompressed;
    }

    let strategy_hub_map = Arc::try_unwrap(strategy_hub_map).unwrap();
    let strategy_hub_map: HashMap<StrategyHubKey, StrategyBranch<TStrategy>> = strategy_hub_map.into_iter().collect();

    if strategy_hub_map.len() == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No strategy hub elements found"));
    }

    println!("Successfully deserialised strategy hub with {} elements (compressed size {} MB, uncompressed size {} MB)", strategy_hub_map.len(), total_strategy_size_bytes_compressed/(1024*1024), total_strategy_size_bytes_uncompressed/(1024*1024));
    Ok(strategy_hub_map)
}
