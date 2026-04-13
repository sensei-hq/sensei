mod types;
mod db;
mod adapters;

fn main() {
    println!("senseid — Sensei indexer daemon");
    println!("Use: senseid start | stop | status | logs");
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn store_opens_in_memory() {
        let store = db::Store::open_memory().unwrap();
        let projects = store.list_projects().unwrap();
        assert!(projects.is_empty());
    }
}
