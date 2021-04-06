use std::collections::{HashMap, HashSet};
use witnet_crypto::hash::calculate_sha256;
use std::convert::TryInto;
use witnet_data_structures::superblock::two_thirds_consensus;
use witnet_data_structures::chain::Hash;
use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64;
use rand::seq::SliceRandom;


// Take size element out of v.len() starting with element at index first plus an offset:
// magic_partition(v, 3, 3, r), v=[0, 1, 2, 3, 4, 5], r=[1].
// Will return elements at index 4, 0, 2.
fn magic_partition_2<T>(v: &[T], first: usize, size: usize, rand_distribution: &[u8]) -> Vec<T>
    where
        T: Clone + Eq + std::hash::Hash,
{
    if first >= v.len() {
        return vec![];
    }

    // Check that the required size is bigger than v
    assert!(size <= v.len());

    let each = v.len() / size;

    let mut step_index = 0_usize;
    let mut step = rand_distribution[step_index] as usize % each;
    let mut a = first;
    let mut b = (a + step) % v.len();
    let mut v_subset = vec![];
    for _ in 0..size {
        v_subset.push(v[b].clone());

        step_index = (step_index + 1) % rand_distribution.len();
        step = rand_distribution[step_index] as usize % each;
        a = (a + each) % v.len();
        b = (a + step) % v.len();
    }

    v_subset
}


// Take size element out of v.len() starting with element at index first plus an offset:
// magic_partition(v, 3, 3, r), v=[0, 1, 2, 3, 4, 5], r=[1].
// Will return elements at index 4, 0, 2.
fn magic_partition_2_hs<T>(v: &[T], first: usize, size: usize, rand_distribution: &[u8]) -> Vec<T>
    where
        T: Clone + Eq + std::hash::Hash,
{
    if first >= v.len() {
        return vec![];
    }

    // Check that the required size is bigger than v
    assert!(size <= v.len());

    let each = v.len() / size;

    let mut step_index = 0_usize;
    let mut step = rand_distribution[step_index] as usize;
    let mut a = first;
    let mut b = (a + step) % v.len();
    let mut hs_subset = HashSet::new();
    for _ in 0..size {
        while hs_subset.contains(&v[b]) {
            b = (b + 1) % v.len();
        }
        hs_subset.insert(v[b].clone());

        step_index = (step_index + 1) % rand_distribution.len();
        step = rand_distribution[step_index] as usize;
        a = (a + each) % v.len();
        b = (a + step) % v.len();
    }

    hs_subset.into_iter().collect()
}

fn magic_partition_3_random<T>(v: &[T], _first: usize, size: usize, rand_distribution: &[u8]) -> Vec<T>
where T: Clone,
{
    let mut rng = Pcg64::from_seed(rand_distribution.try_into().unwrap());
    v.choose_multiple(&mut rng, size).cloned().collect()
}

fn test_magic_partition_2_consensus<F>(mut magic_partition: F) -> Vec<(u32, u64)>
    where F: FnMut(&[i32], usize, usize, &[u8]) -> Vec<i32>,
{
    // Check how many different committees can be generated for a fixed superblock hash
    let mut hist: HashMap<u32, u64> = HashMap::new();
    let random_also_tries: u64 = 100;
    let also_tries: u32 = 1_000_000;
    let tries: u32 = 1_500;
    let v: Vec<i32> = (0..10000).collect();

    for j in 0..random_also_tries {
        let mut rng = Pcg64::seed_from_u64(j);
        let voting_v: HashSet<_> = v.iter().filter_map(|x| {
            // Probability that one identity does not vote
            let p_down = 0.35;
            //let p_down = 0.33114;
            if rng.gen_bool(p_down) {
                None
            } else {
                Some(x)
            }
        }).collect();

        //println!("{} identities will vote", voting_v.len());

        for aaaa in 0..also_tries {
            let superblock_hash = Hash::with_first_u32(aaaa);
            for i in 0..tries {
                // Start counting the members of the subset from:
                // sha256(superblock_hash || superblock_index) % ars_identities.len()
                let superblock_hash_and_index_bytes = [
                    superblock_hash.as_ref(),
                    i.to_be_bytes().as_ref(),
                ]
                    .concat();
                let superblock_hash_and_index_bytes_hashed =
                    Hash::from(calculate_sha256(&superblock_hash_and_index_bytes));
                let first = superblock_hash_and_index_bytes_hashed
                    .div_mod(50)
                    .1 as usize;
                let s = magic_partition(&v, first.try_into().unwrap(), 50, superblock_hash_and_index_bytes_hashed.as_ref());

                let mut votes_count = 0;
                for is in s {
                    if voting_v.contains(&is) {
                        votes_count += 1;
                    }
                }

                if two_thirds_consensus(votes_count, 50) {
                    // Nice!
                    //panic!("Consensus after {} superblocks", i);
                    *hist.entry(i).or_default() += 1;
                    break;
                } else if i == tries - 1 {
                    //panic!("No consensus");
                    *hist.entry(i).or_default() += 1;
                    break;
                } else {
                    // Keep trying
                }
            }
        }

        // Sort histogram by key
        //let v_hist: Vec<_> = hist.iter().sorted_by_key(|(id, x)| **id).collect();
        //panic!("{:?}", v_hist);
    }

    hist.into_iter().collect()
}

fn hist_mean(h: &[(u32, u64)]) -> f64 {
    let mut sum = 0;
    let mut count = 0;

    for (k, v) in h {
        // Add 1 to k because 0 superblocks actually means 1 superblock
        sum += (*k as u64 + 1) * v;
        count += v + 1;
    }

    sum as f64 / count as f64
}

fn main() {
    let hist_new = test_magic_partition_2_consensus(magic_partition_2);
    println!("hist_new = {:?}", hist_new);
    println!("hist_new_mean = {}", hist_mean(&hist_new));
    let hist_old = test_magic_partition_2_consensus(magic_partition_2_hs);
    println!("hist_old = {:?}", hist_old);
    println!("hist_old_mean = {}", hist_mean(&hist_old));
    let hist_rand = test_magic_partition_2_consensus(magic_partition_3_random);
    println!("hist_rand = {:?}", hist_rand);
    println!("hist_rand_mean = {}", hist_mean(&hist_rand));

}
