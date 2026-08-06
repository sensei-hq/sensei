// Graph-scan fixture: two functions with an intra-file call so the scan
// produces a resolvable `calls` edge (compute -> helper).
pub fn compute() -> i32 {
    helper() + 1
}

pub fn helper() -> i32 {
    2
}
