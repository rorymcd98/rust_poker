use concurrent_queue::ConcurrentQueue;
use rand::seq::SliceRandom;

use crate::thread_utils::with_rng;

use super::strategy_branch::{StrategyBranch, StrategyHubElement};
use super::strategy_trait::Strategy;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct StrategyHubBucket<TStrategy: Strategy> {
    pub sb_strategies: Vec<StrategyBranch<TStrategy>>,
    pub bb_strategies: Vec<StrategyBranch<TStrategy>>,
}

// TODO - this needs a total reword. Instead should generate a long list of combinations which we can pop off
#[derive(Debug)]
pub struct StrategyHub<TStrategy: Strategy + Debug> {
    sb_out_queue: ConcurrentQueue<StrategyBranch<TStrategy>>,
    bb_out_queue: ConcurrentQueue<StrategyBranch<TStrategy>>,
    sb_in_queue: ConcurrentQueue<StrategyBranch<TStrategy>>,
    bb_in_queue: ConcurrentQueue<StrategyBranch<TStrategy>>,
    element_take: usize, // How many elements we take from the queue
    element_reserve: usize, // How many elements we reserve in the queue (will increase the amount of shuffling)
}

impl<TStrategy: Strategy + Debug> StrategyHub<TStrategy> {
    pub fn new(num_elements: usize, element_take: usize, element_reserve: usize) -> StrategyHub<TStrategy> {
        if num_elements * 2 < element_take {
            panic!("Number of elements is too small to satisfy the standard take");
        } 
        StrategyHub {
            sb_out_queue: ConcurrentQueue::bounded(num_elements),
            bb_out_queue: ConcurrentQueue::bounded(num_elements),
            sb_in_queue: ConcurrentQueue::bounded(num_elements),
            bb_in_queue: ConcurrentQueue::bounded(num_elements),
            element_take,
            element_reserve,
        }
    }

    fn enough_elements(&self) -> bool {
        self.sb_out_queue.len() > self.element_take + self.element_reserve && self.bb_out_queue.len() > self.element_take + self.element_reserve
    }

    // TODO - This is probably prone to random errors, need to rework

    // Grabs a bunch of strategies for which we will iterate over all combinations of BB SB strategies
    // Might be more efficient to do a stream approach but requires profiling to see
    pub fn get_more_elements(&self) -> StrategyHubBucket<TStrategy> {        
        if self.enough_elements() {
            // println!("hit");
            let mut sb_strategies = Vec::with_capacity(self.element_take);
            let mut bb_strategies = Vec::with_capacity(self.element_take);

            for _ in 0..self.element_take {
                match self.sb_out_queue.pop() {
                    Ok(sb_strategy) => sb_strategies.push(sb_strategy),
                    Err(_) => break,
                }
                match self.bb_out_queue.pop() {
                    Ok(bb_strategy) => bb_strategies.push(bb_strategy),
                    Err(_) => {
                        if let Ok(sb_strategy) = self.sb_out_queue.pop() {
                            self.sb_in_queue.push(sb_strategy).unwrap();
                        }
                        break;
                    }
                }
            }

            return StrategyHubBucket {
                sb_strategies,
                bb_strategies
            }
        } else {
            lazy_static::lazy_static! {
                static ref LOCK: Mutex<()> = Mutex::new(());
                // static ref TOTAL_WAIT_TIME: AtomicU64 = AtomicU64::new(0);
            }

            // let start_time = std::time::Instant::now();
            let _guard = LOCK.lock().unwrap();
            // let duration = start_time.elapsed();
            // TOTAL_WAIT_TIME.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);

            if self.enough_elements() {
                return self.get_more_elements();
            }

            let mut all_sbs: Vec<StrategyBranch<TStrategy>> = Vec::with_capacity(self.sb_in_queue.len() + self.sb_out_queue.len());
            let mut all_bbs: Vec<StrategyBranch<TStrategy>> = Vec::with_capacity(self.bb_in_queue.len() + self.bb_out_queue.len());

            loop {
                if let Ok(sb) = self.sb_in_queue.pop() {
                    all_sbs.push(sb);
                } else {
                    break;
                }
            }

            loop {
                if let Ok(bb) = self.bb_in_queue.pop() {
                    all_bbs.push(bb);
                } else {
                    break;
                }
            }

            loop {
                if let Ok(sb) = self.sb_out_queue.pop() {
                    all_sbs.push(sb);
                } else {
                    break;
                }
            }

            loop {
                if let Ok(bb) = self.bb_out_queue.pop() {
                    all_bbs.push(bb);
                } else {
                    break;
                }
            }
            
            with_rng(|rng|{
                all_sbs.shuffle(rng);
                all_bbs.shuffle(rng);
            });
            
            while let Some(sb) = all_sbs.pop() {
                match self.sb_out_queue.push(sb) {
                    Ok(_) => {},
                    Err(_) => {
                        panic!("Failed to push sb strategy");
                    }
                }
            }

            while let Some(bb) = all_bbs.pop() {
                match self.bb_out_queue.push(bb) {
                    Ok(_) => {},
                    Err(_) => {
                        panic!("Failed to push bb strategy");
                    }
                }
            }

            return self.get_more_elements();
        }
    }

    pub fn return_elements(&self, bucket: StrategyHubBucket<TStrategy>) {
        for sb in bucket.sb_strategies {
            match self.sb_in_queue.push(sb) {
                Ok(_) => {},
                Err(_) => {
                    panic!("Failed to push sb strategy");
                }
            }
        }

        for bb in bucket.bb_strategies {
            match self.bb_in_queue.push(bb) {
                Ok(_) => {},
                Err(_) => {
                    panic!("Failed to push bb strategy");
                }
            }
        }
    }   

    pub fn into_map(self) -> HashMap<StrategyHubElement, StrategyBranch<TStrategy>> {
        let mut map = HashMap::new();
        while let Ok(sb) = self.sb_out_queue.pop() {
            map.insert(sb.strategy_hub_element.clone(), sb);
        }

        while let Ok(bb) = self.bb_out_queue.pop() {
            map.insert(bb.strategy_hub_element.clone(), bb);
        }

        while let Ok(sb) = self.sb_in_queue.pop() {
            map.insert(sb.strategy_hub_element.clone(), sb);
        }

        while let Ok(bb) = self.bb_in_queue.pop() {
            map.insert(bb.strategy_hub_element.clone(), bb);
        }

        map
    }
}