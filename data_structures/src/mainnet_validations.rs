use crate::chain::{Environment, Epoch, PublicKeyHash};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Committee for superblock indices 750-1344
const FIRST_EMERGENCY_COMMITTEE: [&str; 7] = [
    "wit1asdpcspwysf0hg5kgwvgsp2h6g65y5kg9gj5dz",
    "wit13l337znc5yuualnxfg9s2hu9txylntq5pyazty",
    "wit17nnjuxmfuu92l6rxhque2qc3u2kvmx2fske4l9",
    "wit1drcpu0xc2akfcqn8r69vw70pj8fzjhjypdcfsq",
    "wit1cyrlc64hyu0rux7hclmg9rxwxpa0v9pevyaj2c",
    "wit1g0rkajsgwqux9rnmkfca5tz6djg0f87x7ms5qx",
    "wit1etherz02v4fvqty6jhdawefd0pl33qtevy7s4z",
];

/// 22 January 2021 @ 09:00:00 UTC
pub const FIRST_HARD_FORK: Epoch = 192000;
/// 28 April 2021 @ 9:00:00 UTC
pub const SECOND_HARD_FORK: Epoch = 376320;

/// TAPI Engine
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct TapiEngine {
    /// bit votes counter by bits
    pub bit_tapi_counter: BitTapiCounter,
    /// wip activation
    pub wip_activation: HashMap<String, Epoch>,
}

impl TapiEngine {
    pub fn update_bit_counter(
        &mut self,
        v: u32,
        epoch_to_update: Epoch,
        block_epoch: Epoch,
        avoid_wip_list: &HashSet<String>,
    ) {
        // In case of empty epochs, they would be considered as blocks with tapi version to 0
        // In order to not update bit counter from old blocks where the block version was not used,
        // the first time (bit_tapi_counter.last_epoch == 0) would be skipped in this conditional branch
        if self.bit_tapi_counter.last_epoch != 0
            && epoch_to_update > self.bit_tapi_counter.last_epoch + 1
        {
            let init = self.bit_tapi_counter.last_epoch + 1;
            let end = epoch_to_update;
            for i in init..end {
                self.update_bit_counter(0, i, block_epoch, avoid_wip_list);
            }
        }
        for n in 0..self.bit_tapi_counter.len() {
            if let Some(mut bit_counter) = self.bit_tapi_counter.get_mut(n, &epoch_to_update) {
                if !self.wip_activation.contains_key(&bit_counter.wip)
                    && !avoid_wip_list.contains(&bit_counter.wip)
                {
                    if is_bit_n_activated(v, n) {
                        bit_counter.votes += 1;
                    }
                    if (epoch_to_update - bit_counter.init) % bit_counter.period == 0 {
                        if (bit_counter.votes * 100) / bit_counter.period >= 80 {
                            // An offset of 21 is added to ensure that the activation of the WIP is
                            // achieved with consolidated blocks
                            self.wip_activation
                                .insert(bit_counter.wip.clone(), block_epoch + 21);
                        }
                        bit_counter.votes = 0;
                    }
                }
            }
        }
        self.bit_tapi_counter.last_epoch = epoch_to_update;
    }

    pub fn initialize_wip_information(
        &mut self,
        environment: Environment,
    ) -> (Epoch, HashSet<String>) {
        let mut voting_wips = vec![None; 32];

        match environment {
            Environment::Mainnet => {
                // Hardcoded information about WIPs
                self.wip_activation
                    .insert("WIP0008".to_string(), FIRST_HARD_FORK);
                self.wip_activation
                    .insert("WIP0009-0011-0012".to_string(), SECOND_HARD_FORK);

                // Hardcoded information about WIPs in vote processing
                let bit = 0;
                let wip_0014 = BitVotesCounter {
                    votes: 0,
                    period: 26880,
                    wip: "WIP0014".to_string(),
                    init: 500000,
                    end: u32::MAX,
                    bit,
                };
                voting_wips[bit] = Some(wip_0014);
            }
            Environment::Testnet | Environment::Development => {
                // In non-mainnet chains, all the WIPs that are active in mainnet are considered
                // active since epoch 0. And there is no voting.
                self.wip_activation.insert("WIP0008".to_string(), u32::MAX);
                self.wip_activation
                    .insert("WIP0009-0011-0012".to_string(), u32::MAX);

                // Hardcoded information about WIPs in vote processing
                let bit = 0;
                let wip_0014 = BitVotesCounter {
                    votes: 0,
                    period: 120, // 120*30sec (1hour)
                    wip: "WIP0014".to_string(),
                    init: 5200,
                    end: u32::MAX,
                    bit,
                };
                voting_wips[bit] = Some(wip_0014);
            }
        };

        // Assessment of new WIPs
        let mut min_epoch = u32::MAX;
        let mut old_wips = HashSet::default();

        for (i, wip) in voting_wips.into_iter().enumerate() {
            match wip {
                Some(wip) => {
                    if self.bit_tapi_counter.contains(i, &wip.wip) {
                        old_wips.insert(wip.wip.clone());
                    } else {
                        if wip.init < min_epoch {
                            min_epoch = wip.init;
                        }
                        self.bit_tapi_counter.insert(wip.clone());
                    }
                }
                None => self.bit_tapi_counter.remove(i),
            }
        }

        (min_epoch, old_wips)
    }

    pub fn in_voting_range(&self, epoch: Epoch, wip: &str) -> bool {
        for i in 0..self.bit_tapi_counter.len() {
            if let Some(bit_info) = self.bit_tapi_counter.get(i, &epoch) {
                if bit_info.wip == wip {
                    return true;
                }
            }
        }

        false
    }
}

/// Struct that count the positives votes of a WIP
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct BitVotesCounter {
    pub votes: u32,
    pub period: Epoch,
    pub wip: String,
    pub init: Epoch,
    pub end: Epoch,
    pub bit: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct BitTapiCounter {
    info: [Option<BitVotesCounter>; 32],
    last_epoch: Epoch,
    current_length: usize,
}

impl BitTapiCounter {
    pub fn get(&self, bit: usize, epoch: &u32) -> Option<&BitVotesCounter> {
        match self.info.get(bit) {
            Some(Some(bit_info)) => {
                if *epoch >= bit_info.init && *epoch < bit_info.end {
                    Some(bit_info)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, bit: usize, epoch: &u32) -> Option<&mut BitVotesCounter> {
        match self.info.get_mut(bit) {
            Some(Some(bit_info)) => {
                if *epoch >= bit_info.init && *epoch < bit_info.end {
                    Some(bit_info)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn insert(&mut self, bit_info: BitVotesCounter) {
        let k = bit_info.bit;
        if k >= self.info.len() {
            log::error!(
                "Tapi Engine: This bit position ({}) is invalid. {} has not been included",
                k,
                bit_info.wip
            );
        } else {
            self.info[k] = Some(bit_info);

            if k >= self.current_length {
                self.current_length = k + 1;
            }
        }
    }

    pub fn remove(&mut self, bit: usize) {
        if bit >= self.info.len() {
            log::error!("Tapi Engine: This bit position ({}) is invalid", bit,);
        } else {
            self.info[bit] = None;

            if bit + 1 == self.current_length {
                self.update_current_length();
            }
        }
    }

    pub fn update_current_length(&mut self) {
        let mut length = 0;
        for bit_info in self.info.iter().flatten() {
            length = bit_info.bit + 1;
        }

        self.current_length = length;
    }

    pub fn contains(&self, bit: usize, wip: &str) -> bool {
        match self.info.get(bit) {
            Some(Some(bit_info)) => bit_info.wip == wip,
            _ => false,
        }
    }

    pub fn len(&self) -> usize {
        self.current_length
    }

    pub fn is_empty(&self) -> bool {
        self.current_length == 0
    }

    pub fn info(&self, active_wips: &HashMap<String, Epoch>) -> Vec<BitVotesCounter> {
        self.info[..self.current_length]
            .iter()
            .filter_map(|x| {
                if let Some(bit_info) = x {
                    if active_wips.contains_key(&bit_info.wip) {
                        None
                    } else {
                        Some(bit_info)
                    }
                } else {
                    None
                }
            })
            .cloned()
            .collect()
    }

    pub fn last_epoch(&self) -> Epoch {
        self.last_epoch
    }
}

fn is_bit_n_activated(v: u32, n: usize) -> bool {
    v & (1 << n) != 0
}

/// Return a hard-coded signing committee if the provided epoch belongs to an emergency period.
/// 750 and 1344: Between those indices, a special committee of 7 nodes was set.
pub fn in_emergency_period(
    superblock_index: Epoch,
    environment: Environment,
) -> Option<Vec<PublicKeyHash>> {
    if Environment::Mainnet == environment && superblock_index > 750 && superblock_index < 1344 {
        Some(
            FIRST_EMERGENCY_COMMITTEE
                .iter()
                .map(|address| address.parse().expect("Malformed signing committee"))
                .collect(),
        )
    } else {
        None
    }
}

/// Returns a boolean indicating whether the epoch provided is after the first hard fork date
pub fn after_first_hard_fork(epoch: Epoch, environment: Environment) -> bool {
    match environment {
        Environment::Mainnet => epoch >= FIRST_HARD_FORK,
        Environment::Testnet | Environment::Development => false,
    }
}

/// Returns a boolean indicating whether the epoch provided is after the second hard fork date
pub fn after_second_hard_fork(epoch: Epoch, environment: Environment) -> bool {
    match environment {
        Environment::Mainnet => epoch >= SECOND_HARD_FORK,
        Environment::Testnet | Environment::Development => false,
    }
}

/// Allows to check the active Witnet Improvement Proposals
#[derive(Clone, Debug)]
pub struct ActiveWips {
    pub active_wips: HashMap<String, Epoch>,
    pub block_epoch: Epoch,
}

impl ActiveWips {
    // WIP 0008 was activated through community coordination on January 22, 2021
    pub fn wip_0008(&self) -> bool {
        self.active_wips
            .get("WIP0008")
            .map(|activation_epoch| self.block_epoch >= *activation_epoch)
            .unwrap_or(false)
    }

    // WIPs 0009, 0011 and 0012 were activated through community coordination on April 28, 2021
    pub fn wips_0009_0011_0012(&self) -> bool {
        self.active_wips
            .get("WIP0009-0011-0012")
            .map(|activation_epoch| self.block_epoch >= *activation_epoch)
            .unwrap_or(false)
    }

    pub fn wip0014(&self) -> bool {
        self.active_wips
            .get("WIP0014")
            .map(|activation_epoch| self.block_epoch >= *activation_epoch)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_emergency_period_not_mainnet() {
        assert_eq!(in_emergency_period(1300, Environment::Testnet), None)
    }
    #[test]
    fn test_in_emergency_period_not_inside_period() {
        assert_eq!(in_emergency_period(50, Environment::Mainnet), None)
    }
    #[test]
    fn test_in_emergency_period_inside_first_emergency_period() {
        assert_eq!(
            in_emergency_period(800, Environment::Mainnet),
            Some(
                FIRST_EMERGENCY_COMMITTEE
                    .iter()
                    .map(|address| address.parse().expect("Malformed signing committee"))
                    .collect(),
            )
        )
    }

    #[test]
    fn test_is_bit_n_activated() {
        let aux = 1;
        assert!(is_bit_n_activated(aux, 0));
        assert!(!is_bit_n_activated(aux, 1));

        let aux = 2;
        assert!(!is_bit_n_activated(aux, 0));
        assert!(is_bit_n_activated(aux, 1));

        let aux = 3;
        assert!(is_bit_n_activated(aux, 0));
        assert!(is_bit_n_activated(aux, 1));
    }

    #[test]
    fn test_bit_tapi_counter() {
        let mut tapi_counter = BitTapiCounter::default();
        assert!(tapi_counter.is_empty());

        let mut aux = BitVotesCounter::default();
        aux.init = 0;
        aux.end = 50;
        aux.wip = "Wip1".to_string();
        aux.bit = 0;
        tapi_counter.insert(aux);
        assert!(!tapi_counter.is_empty());
        assert!(tapi_counter.get(0, &100).is_none());
        assert!(tapi_counter.contains(0, &"Wip1".to_string()));
        assert!(!tapi_counter.contains(1, &"Wip1".to_string()));
        assert_eq!(tapi_counter.current_length, 1);

        let mut aux2 = BitVotesCounter::default();
        aux2.init = 75;
        aux2.end = 125;
        aux2.wip = "Wip2".to_string();
        aux2.bit = 0;
        tapi_counter.insert(aux2);
        assert_eq!(tapi_counter.get(0, &100).unwrap().wip, "Wip2".to_string());
        assert!(tapi_counter.get(1, &100).is_none());
        assert!(tapi_counter.contains(0, &"Wip2".to_string()));
        assert_eq!(tapi_counter.current_length, 1);

        assert_eq!(tapi_counter.get(0, &100).unwrap().votes, 0);
        let mut votes_counter = tapi_counter.get_mut(0, &100).unwrap();
        votes_counter.votes += 1;
        assert_eq!(tapi_counter.get(0, &100).unwrap().votes, 1);

        tapi_counter.remove(0);
        assert_eq!(tapi_counter.current_length, 0);
    }

    #[test]
    fn test_bit_tapi_counter_invalid_bit() {
        let mut tapi_counter = BitTapiCounter::default();
        assert!(tapi_counter.is_empty());

        let mut aux = BitVotesCounter::default();
        aux.init = 0;
        aux.end = 50;
        aux.wip = "Wip1".to_string();
        aux.bit = 32;
        tapi_counter.insert(aux);
        assert!(tapi_counter.is_empty());

        let mut aux = BitVotesCounter::default();
        aux.init = 0;
        aux.end = 50;
        aux.wip = "Wip1".to_string();
        aux.bit = 0;
        tapi_counter.insert(aux);
        assert_eq!(tapi_counter.current_length, 1);

        tapi_counter.remove(32);
        assert_eq!(tapi_counter.current_length, 1);
    }

    #[test]
    fn test_update_bit_counter() {
        let empty_hs = HashSet::default();
        let mut t = TapiEngine::default();
        let bit = 0;
        let wip = BitVotesCounter {
            votes: 0,
            period: 100,
            wip: "test0".to_string(),
            init: 10_000,
            end: 20_000,
            bit,
        };
        t.bit_tapi_counter.insert(wip);
        assert_eq!(t.bit_tapi_counter.last_epoch, 0);

        t.update_bit_counter(1, 9_999, 9_999, &empty_hs);
        // Updating with epoch < init does not increase the votes counter
        assert_eq!(t.bit_tapi_counter.info[bit].clone().unwrap().votes, 0);
        assert_eq!(t.bit_tapi_counter.last_epoch, 9_999);

        t.update_bit_counter(1, 10_000, 10_000, &empty_hs);
        // Updating with epoch >= init does increase the votes counter
        // But since this is the first epoch, the votes counter is reset to 0 again afterwards
        assert_eq!(t.bit_tapi_counter.info[bit].clone().unwrap().votes, 0);

        t.update_bit_counter(1, 10_001, 10_001, &empty_hs);
        // Updating with epoch >= init does increase the votes counter
        assert_eq!(t.bit_tapi_counter.info[bit].clone().unwrap().votes, 1);

        t.update_bit_counter(1, 10_002, 10_002, &empty_hs);
        // Updating with epoch >= init does increase the votes counter
        assert_eq!(t.bit_tapi_counter.info[bit].clone().unwrap().votes, 2);

        // Updating with an epoch that was already updated will count the votes twice, there is no
        // protection against this because the update_new_wip_votes function must be able to count
        // votes from old blocks
        /*
        t.update_bit_counter(1, 10_002, &empty_hs);
         */

        t.update_bit_counter(0, 10_003, 10_003, &empty_hs);
        // Updating with epoch >= init but voting against does not increase the votes counter
        assert_eq!(t.bit_tapi_counter.info[bit].clone().unwrap().votes, 2);

        t.update_bit_counter(0, 10_103, 10_103, &empty_hs);
        // The vote counting is at epoch 10_100, the votes should be reset to 0
        assert_eq!(t.bit_tapi_counter.info[bit].clone().unwrap().votes, 0);

        // Add 90 votes to test activation
        for epoch in 10_200..10_290 {
            t.update_bit_counter(1, epoch, epoch, &empty_hs);
        }
        // More than 80% of votes means that the WIP should activate at the next counting epoch
        assert_eq!(t.bit_tapi_counter.info[bit].clone().unwrap().votes, 89);

        // Simulate large block gap, instead of updating at 10_300 update at 10_500
        // TODO: block 10_500 will be validated with the new WIP disabled, same as all superblocks
        // or other logic. But the activation date of the WIP will be 10_321. Fix the update process
        // so that all the blocks after 10_321 are validated using the new validation logic, or
        // change the WIP activation date to 10_501?
        // Decided to change the WIP activation date to 10_521
        t.update_bit_counter(0, 10_500, 10_500, &empty_hs);
        // The votes counter should reset
        assert_eq!(t.bit_tapi_counter.info[bit].clone().unwrap().votes, 0);
        // The activation date should be
        assert_eq!(*t.wip_activation.get("test0").unwrap(), 10_500 + 21);
    }

    #[test]
    fn test_update_bit_counter_multi_vote() {
        let empty_hs = HashSet::default();
        let mut t = TapiEngine::default();
        let wip0 = BitVotesCounter {
            votes: 0,
            period: 100,
            wip: "test0".to_string(),
            init: 10_000,
            end: 20_000,
            bit: 0,
        };
        let wip1 = BitVotesCounter {
            votes: 0,
            period: 100,
            wip: "test1".to_string(),
            init: 10_000,
            end: 20_000,
            bit: 1,
        };
        t.bit_tapi_counter.insert(wip0);
        t.bit_tapi_counter.insert(wip1);
        assert_eq!(t.bit_tapi_counter.last_epoch, 0);

        // Vote for none
        t.update_bit_counter(0, 10_001, 10_001, &empty_hs);
        assert_eq!(t.bit_tapi_counter.info[0].clone().unwrap().votes, 0);
        assert_eq!(t.bit_tapi_counter.info[1].clone().unwrap().votes, 0);
        assert_eq!(t.bit_tapi_counter.last_epoch, 10_001);

        // Vote for both
        t.update_bit_counter(3, 10_002, 10_002, &empty_hs);
        assert_eq!(t.bit_tapi_counter.info[0].clone().unwrap().votes, 1);
        assert_eq!(t.bit_tapi_counter.info[1].clone().unwrap().votes, 1);

        // Vote only for wip0
        t.update_bit_counter(1, 10_002, 10_002, &empty_hs);
        assert_eq!(t.bit_tapi_counter.info[0].clone().unwrap().votes, 2);
        assert_eq!(t.bit_tapi_counter.info[1].clone().unwrap().votes, 1);

        // Vote only for wip1
        t.update_bit_counter(2, 10_002, 10_002, &empty_hs);
        assert_eq!(t.bit_tapi_counter.info[0].clone().unwrap().votes, 2);
        assert_eq!(t.bit_tapi_counter.info[1].clone().unwrap().votes, 2);

        // Add 90 votes to test activation of both wips in the same epoch
        for epoch in 10_003..10_093 {
            t.update_bit_counter(3, epoch, epoch, &empty_hs);
        }

        assert_eq!(t.bit_tapi_counter.info[0].clone().unwrap().votes, 92);
        assert_eq!(t.bit_tapi_counter.info[1].clone().unwrap().votes, 92);

        t.update_bit_counter(0, 10_100, 10_100, &empty_hs);
        // The votes counter should reset
        assert_eq!(t.bit_tapi_counter.info[0].clone().unwrap().votes, 0);
        assert_eq!(t.bit_tapi_counter.info[1].clone().unwrap().votes, 0);
        // The activation date should be current + 21
        assert_eq!(*t.wip_activation.get("test0").unwrap(), 10_100 + 21);
        assert_eq!(*t.wip_activation.get("test1").unwrap(), 10_100 + 21);
    }

    #[test]
    fn test_update_bit_counter_future_wip() {
        // Check that voting for unallocated wips is allowed, but the extra votes are not counted,
        // and the votes for active bits are valid
        let empty_hs = HashSet::default();
        let mut t = TapiEngine::default();
        let bit = 0;
        let wip = BitVotesCounter {
            votes: 0,
            period: 100,
            wip: "test0".to_string(),
            init: 10_000,
            end: 20_000,
            bit,
        };
        t.bit_tapi_counter.insert(wip);
        assert_eq!(t.bit_tapi_counter.info[bit].clone().unwrap().votes, 0);

        // Vote "yes" to all the 32 bits, even though there is only 1 active wip (bit 0)
        t.update_bit_counter(u32::MAX, 10_001, 10_001, &empty_hs);
        // This is a valid block and a valid vote
        assert_eq!(t.bit_tapi_counter.info[bit].clone().unwrap().votes, 1);
    }

    #[test]
    fn test_initialize_wip_information() {
        let mut t = TapiEngine::default();

        let (epoch, old_wips) = t.initialize_wip_information(Environment::Mainnet);
        // The first block whose vote must be counted is the one from WIP0014
        let init_epoch_wip0014 = 500000;
        assert_eq!(epoch, init_epoch_wip0014);
        // The TapiEngine was just created, there list of old_wips must be empty
        assert_eq!(old_wips, HashSet::new());
        // The list of active WIPs only contains the first and the second hard fork
        let mut hm = HashMap::new();
        hm.insert("WIP0008".to_string(), FIRST_HARD_FORK);
        hm.insert("WIP0009-0011-0012".to_string(), SECOND_HARD_FORK);
        assert_eq!(t.wip_activation, hm);

        // Test initialize_wip_information with a non-empty TapiEngine
        let (epoch, old_wips) = t.initialize_wip_information(Environment::Mainnet);
        // WIP0014 is already included and it won't be updated
        let name_wip0014 = "WIP0014".to_string();
        let mut hs = HashSet::new();
        hs.insert(name_wip0014);
        assert_eq!(old_wips, hs);

        // There is no new WIPs to update so we obtain the max value
        assert_eq!(epoch, u32::MAX);
    }

    #[test]
    fn test_initialize_mainnet_and_testnet() {
        let mut t_mainnet = TapiEngine::default();
        let (_epoch, _old_wips) = t_mainnet.initialize_wip_information(Environment::Mainnet);

        let mut t_testnet = TapiEngine::default();
        let (_epoch, _old_wips) = t_testnet.initialize_wip_information(Environment::Testnet);

        // The keys of the wip_activation map should be the same
        assert_eq!(
            t_testnet.wip_activation.keys().collect::<HashSet<_>>(),
            t_mainnet.wip_activation.keys().collect::<HashSet<_>>(),
        )
    }
}
