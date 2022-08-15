
///trait VRNG defines method signatures for generating 
///random numbers of all u types
pub trait VRNG {
    fn generate_u8(&mut self) -> u8;
    fn generate_u16(&mut self) -> u16;
    fn generate_u32(&mut self) -> u32;
    fn generate_u64(&mut self) -> u64;
    fn generate_u128(&mut self) -> u128;
    fn generate_usize(&mut self) -> usize;
    fn generate_word(&mut self) -> String;
    fn generate_words(&mut self, n: usize) -> Vec<String> {
        let mut vec: Vec<String> = Vec::new();
        let mut i: usize = 0;
        while i < n {
            vec.push(self.generate_word());
            i += 1;
        }
        vec
    }
    fn generate_phrase(&mut self, n: usize) -> String {
        let vec: Vec<String> = self.generate_words(n);
        let mut phrase: String = vec.join ("");
        return phrase.trim_start().to_string();
    }
}
