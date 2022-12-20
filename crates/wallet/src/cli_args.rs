
use clap::{Args, Parser};


#[derive(Debug, Parser)]
#[clap(author, version, about)]

pub struct WalletArgs{
    ///first argument!
    pub arg_one: String, 
    ///second argument!
    pub arg_two: String,

}

