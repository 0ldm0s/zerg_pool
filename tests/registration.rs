use zerg_pool::{DronePool, Process, RegistrationError};

#[test]
fn test_register_drone() {
    let mut pool = DronePool::new("tcp://127.0.0.1:9001".to_string());
    let drone = Process {
        id: "worker-1".to_string(),
        endpoint: "127.0.0.1:8080".to_string(),
        capability: vec!["compute".to_string()],
    };

    assert!(pool.register_drone(drone).is_ok());
    assert_eq!(pool.get_worker_count(), 1);
}

#[test]
fn test_invalid_endpoint() {
    let mut pool = DronePool::new("tcp://127.0.0.1:9001".to_string());
    let drone = Process {
        id: "worker-1".to_string(),
        endpoint: "invalid_address".to_string(),
        capability: vec![],
    };

    assert!(matches!(
        pool.register_drone(drone),
        Err(RegistrationError::InvalidEndpoint)
    ));
}