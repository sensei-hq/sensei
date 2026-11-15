//! Network policy shared by every outbound client that must reach only the public
//! internet: the address rule ([`addr`]) and a DNS resolver that enforces it
//! ([`resolver`]).

pub mod addr;
pub mod resolver;
