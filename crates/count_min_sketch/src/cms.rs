#![allow(unused)]
use std::hash::{BuildHasher, Hash, Hasher};
use std::collections::hash_map::RandomState;
use std::ops::{AddAssign, SubAssign};
use siphasher::sip::SipHasher13;
use std::num::Wrapping;

/// CountMinSketch is a probabilistic data structure for estimating 
/// values, typically frequencies in a data stream. This is implemented
/// in the VRRB protocol to provide a fast, scalable, dynamic data 
/// structure for storing and estimating the reputation score of nodes
/// in the network and message credits of nodes in the network which
/// are two of the core security features of the VRRB protocol.
/// While it is subject to overestimation, the VRRB protocol accounts
/// for such maximum probabilistic overestimation by bucketizing 
/// reputation scores and calculating nodes required stake to become 
/// a validator based on the bucket they fall into. The buckets account
/// for some overestimation and round down, sometimes requiring nodes to 
/// put up a larger stake than if the data structure were perfectly accurate
/// however, this is a positive tradeoff, as the speed and scalability of 
/// tracking message credits and reputations improves, and in the event that
/// there is an overestimation we ensure that we are not rewarding nodes 
/// for reputation they haven't truly earned. For example:
///
/// Nodes with reputation between 0 and 100 may have an estimate in CountMinSketch
/// as high as 150, so all nodes with reputation estimate between 0 and 150 are in 
/// the same bucket, of requiring the maximum stake. While nodes with reputation
/// estimates between 151 and 250 are in the 2nd bucket, and so forth and so on.
/// Nodes can now also quickly agree on the buckets which their peers are in for
/// the purpose of calculating required stake of each validator and determining
/// their current eligibility for election to a farmer or harvester quorum.
pub struct CountMinSketch<T> 
where
    T: AddAssign + SubAssign
{
    width: usize,
    depth: usize,
    matrix: Vec<Vec<T>>,
    hash_builder: RandomState,
    //TODO: Add max and min values
}

impl<T> CountMinSketch<T> 
where
    T: AddAssign + SubAssign + Hash + Default + Copy + Ord
{
    /// Creates a new CountMinSketch struct 
    pub fn new(width: usize, depth: usize) -> Self {
        let matrix = vec![vec![T::default(); width]; depth];
        let hash_builder = RandomState::new();

        CountMinSketch {
            width,
            depth,
            matrix,
            hash_builder,
        }
    }

    /// Takes a reference to an item implementing the `Hash` trait
    /// and a index representing the hash function. It creates a new
    /// hasher using the hash_builder, hashes the item, and returns
    /// the hashed value modulo the width of the sketch matric.
    fn hash_pair(&self, item: &impl Hash, index: usize) -> usize {
        let mut hasher = self.hash_builder.build_hasher();
        let wrapping_index = Wrapping(index as u64);
        let wrapping_hash = Wrapping(hasher.finish());

        (wrapping_hash + wrapping_index).0 as usize % self.width
    }

    /// Takes a reference to an item that implements `Hash` and
    /// returns a vector hashed values for each hash function
    /// (one value for each row in the sketch matrix)
    fn hash_functions(&self, item: &impl Hash) -> Vec<usize> {
        (0..self.depth).map(|i| self.hash_pair(item, i)).collect()
    }

    /// Takes a reference to an item implementing `Hash` and 
    /// a value of type `T` to be added. It calculates the hashes
    /// for the item using `hash_functions` method, and updates
    /// the sketch matrix adding the given value at respective
    /// positions
    ///
    /// # Examples
    ///
    /// ```
    /// // Need some float primitives to calculate 
    /// // depth and width based on desired error_bound and probability
    /// use std::f64::consts::E;
    /// use std::f64::ceil;
    /// use std::f64::ln;
    /// use cms::cms::CountMinSketch;
    /// 
    /// // Calculate the width and depth based on desired error bound
    /// let error_bound = 50f64;
    /// let probability = 0.0001f64;
    /// let width = ceil(E / error_bound) as usize;
    /// let depth = ceil(ln(1.0 / probability)) as usize;
    ///
    /// // Create CountMinSketch with calculated depth and width
    /// let mut cms = CountMinSketch::<i64>::new(width, depth);
    ///
    /// // Create a mock node_id
    /// let node_id = "node_1";
    ///
    /// // Increment the node's reputation score;
    /// cms.increment(&node_id, 100);
    /// let estimated_score = cms.estimate(&node_id);
    /// println!("Estimated reputation score after increment: {}", estimated_score);
    ///
    /// // Decrement the reputation score
    /// cms.increment(&node_id, -50);
    /// let estmimated_score = cms.estimate(&node_id);
    /// println!("Estimated reputation score after decrement: {}", estimated_score);
    ///
    ///
    /// ```
    pub fn increment(&mut self, item: &impl Hash, value: T) {
        let hashes = self.hash_functions(item);
        (0..self.depth).into_iter()
            .for_each(|i| {
                self.matrix[i][hashes[i]] += value;
            }
        )
    }

    /// Takes a reference to an item implementing `Hash` and
    /// returns an estimate of the value for that item. It calculates
    /// the hash values for the item using hash_functions and returns 
    /// the minimum value found at the respective positions in the sketch
    /// matrix.
    pub fn estimate(&self, item: &impl Hash) -> T {
        let hashes = self.hash_functions(item);
        let mut min_estimate = self.matrix[0][hashes[0]];
        (1..self.depth).into_iter().for_each(|i| {
            min_estimate = std::cmp::min(min_estimate, self.matrix[i][hashes[i]]); 
        });

        min_estimate
    }
}
