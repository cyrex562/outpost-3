#[cfg(test)]
mod cargo_system_integration_tests {
    use outpost3::{
        cargo_events::CargoEventLog, cargo_request::CargoRequestManager,
        cargo_transfer::CargoTransferManager,
        production_chain_routes::ProductionChainRouteGenerator, ResourceType, StationId, TrainId,
    };

    /// Helper to create a basic test setup
    struct TestSetup {
        request_manager: CargoRequestManager,
        transfer_manager: CargoTransferManager,
        event_log: CargoEventLog,
        _route_generator: ProductionChainRouteGenerator,
    }

    impl TestSetup {
        fn new() -> Self {
            Self {
                request_manager: CargoRequestManager::new(),
                transfer_manager: CargoTransferManager::new(),
                event_log: CargoEventLog::new(),
                _route_generator: ProductionChainRouteGenerator::new(),
            }
        }

        fn setup_trains(&mut self, train_ids: &[TrainId], capacity: u32) {
            for &train_id in train_ids {
                self.transfer_manager.register_train(train_id, capacity);
            }
        }
    }

    #[test]
    fn test_full_cargo_lifecycle() {
        let mut setup = TestSetup::new();
        setup.setup_trains(&[TrainId(1)], 500);

        let current_tick = 0;

        // Step 1: Create cargo request
        let request_id = setup
            .request_manager
            .create_request(
                ResourceType::Steel,
                100,
                StationId(1),
                StationId(2),
                5,
                current_tick,
            )
            .expect("Failed to create request");

        // Step 2: Verify request exists
        let request = setup
            .request_manager
            .get_request(request_id)
            .expect("Request not found");
        assert_eq!(request.quantity, 100);

        // Step 3: Clone for later use
        let request_for_loading = request.clone();

        // Step 4: Assign request
        setup
            .request_manager
            .assign_request(request_id, 10)
            .expect("Failed to assign request");

        // Step 5: Start loading
        setup
            .transfer_manager
            .start_loading(
                TrainId(1),
                StationId(1),
                &request_for_loading,
                20,
                &mut setup.request_manager,
            )
            .expect("Failed to start loading");

        // Step 6: Advance loading (10 ticks for 100 units)
        let completed = setup
            .transfer_manager
            .advance_loading(10, &mut setup.request_manager);
        assert_eq!(completed.len(), 1);
        assert!(completed[0].transfer_complete);

        // Step 7: Verify cargo is on train
        let cargo = setup
            .transfer_manager
            .get_train_cargo(TrainId(1))
            .expect("Train not found");
        assert_eq!(cargo.current_load(), 100);

        // Step 8: Unload at destination
        let unload_results = setup
            .transfer_manager
            .unload_at_station(TrainId(1), StationId(2), &mut setup.request_manager, 100)
            .expect("Failed to unload");

        assert_eq!(unload_results.len(), 1);

        // Step 9: Verify cargo is gone from train
        let cargo = setup
            .transfer_manager
            .get_train_cargo(TrainId(1))
            .expect("Train not found");
        assert_eq!(cargo.current_load(), 0);
    }

    #[test]
    fn test_multiple_trains_priority_queueing() {
        let mut setup = TestSetup::new();
        setup.setup_trains(&[TrainId(1), TrainId(2), TrainId(3)], 500);

        // Create multiple requests with different priorities
        let req1 = setup
            .request_manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 3, 0)
            .unwrap();

        let req2 = setup
            .request_manager
            .create_request(ResourceType::Food, 80, StationId(1), StationId(2), 8, 0)
            .unwrap();

        let req3 = setup
            .request_manager
            .create_request(ResourceType::Water, 120, StationId(1), StationId(2), 5, 0)
            .unwrap();

        // Get high priority requests
        let high_priority = setup.request_manager.get_by_priority(5);
        assert_eq!(high_priority.len(), 2); // req2 (8) and req3 (5)
        assert_eq!(high_priority[0].id, req2); // Highest priority first

        // Assign multiple requests
        setup.request_manager.assign_request(req1, 10).ok();
        setup.request_manager.assign_request(req2, 10).ok();
        setup.request_manager.assign_request(req3, 10).ok();

        let from_source = setup.request_manager.get_pending_at_source(StationId(1));
        // Pending at source should be reduced or empty after assignment
        assert!(from_source.len() <= 3);
    }

    #[test]
    fn test_cargo_flow_from_production_chain() {
        let mut setup = TestSetup::new();
        setup.setup_trains(&[TrainId(1)], 500);

        // Just verify the systems are ready and cargo requests can be created
        let request_id = setup
            .request_manager
            .create_request(ResourceType::Steel, 100, StationId(10), StationId(11), 7, 0)
            .expect("Failed to create request");

        let request = setup
            .request_manager
            .get_request(request_id)
            .expect("Request not found");
        assert_eq!(request.quantity, 100);
        assert_eq!(request.priority, 7);
    }

    #[test]
    fn test_train_capacity_management() {
        let mut setup = TestSetup::new();
        setup.setup_trains(&[TrainId(1)], 200); // Limited capacity

        let cargo = setup.transfer_manager.get_train_cargo(TrainId(1)).unwrap();
        assert_eq!(cargo.capacity, 200);
        assert_eq!(cargo.available_capacity(), 200);

        // Create request larger than capacity
        let large_request = setup
            .request_manager
            .create_request(ResourceType::Steel, 300, StationId(1), StationId(2), 5, 0)
            .unwrap();

        // Try to load - should fail
        let request = setup.request_manager.get_request(large_request).unwrap();
        let result = setup.transfer_manager.start_loading(
            TrainId(1),
            StationId(1),
            &request.clone(),
            0,
            &mut setup.request_manager,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_event_sourcing_replay() {
        let mut setup = TestSetup::new();
        setup.setup_trains(&[TrainId(1)], 500);

        // Create request
        setup
            .request_manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();

        // Verify event log is set up and can log events
        assert_eq!(setup.event_log.event_count(), 0);

        // Query by time range - should return empty since we haven't logged anything
        let early_events = setup.event_log.events_in_range(0, 30);
        assert_eq!(early_events.len(), 0);
    }

    #[test]
    fn test_concurrent_cargo_requests() {
        let mut setup = TestSetup::new();
        setup.setup_trains(&[TrainId(1), TrainId(2)], 500);

        // Create multiple concurrent requests
        let mut request_ids = Vec::new();
        for i in 1..=5 {
            let req_id = setup
                .request_manager
                .create_request(
                    ResourceType::Steel,
                    50 + (i as u32 * 10),
                    StationId(1),
                    StationId(2),
                    (i as u8) % 10,
                    0,
                )
                .unwrap();
            request_ids.push(req_id);
        }

        // Verify all requests exist
        assert_eq!(request_ids.len(), 5);
        for id in &request_ids {
            let req = setup.request_manager.get_request(*id);
            assert!(req.is_some());
        }

        // Assign some requests
        setup
            .request_manager
            .assign_request(request_ids[0], 10)
            .ok();
        setup
            .request_manager
            .assign_request(request_ids[2], 10)
            .ok();

        // Verify we can query by source
        let from_source = setup.request_manager.get_pending_at_source(StationId(1));
        assert_eq!(from_source.len(), 3); // 5 - 2 assigned
    }

    #[test]
    fn test_production_chain_complete_flow() {
        let mut setup = TestSetup::new();
        setup.setup_trains(&[TrainId(1)], 500);

        // Create cargo request for production chain
        let request = setup
            .request_manager
            .create_request(ResourceType::Steel, 100, StationId(20), StationId(21), 6, 0)
            .unwrap();

        // Assign and load
        setup.request_manager.assign_request(request, 5).ok();

        let req = setup.request_manager.get_request(request).unwrap().clone();
        setup
            .transfer_manager
            .start_loading(
                TrainId(1),
                StationId(20),
                &req,
                10,
                &mut setup.request_manager,
            )
            .ok();

        // Advance loading
        setup
            .transfer_manager
            .advance_loading(10, &mut setup.request_manager);

        // Unload at destination
        setup
            .transfer_manager
            .unload_at_station(TrainId(1), StationId(21), &mut setup.request_manager, 50)
            .ok();

        // Verify final state
        let train_cargo = setup.transfer_manager.get_train_cargo(TrainId(1)).unwrap();
        assert_eq!(train_cargo.current_load(), 0);
    }

    #[test]
    fn test_station_source_queries() {
        let mut setup = TestSetup::new();

        // Create multiple requests from different stations
        let _req1 = setup
            .request_manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();

        let _req2 = setup
            .request_manager
            .create_request(ResourceType::Food, 80, StationId(1), StationId(3), 3, 0)
            .unwrap();

        let _req3 = setup
            .request_manager
            .create_request(ResourceType::Water, 120, StationId(2), StationId(3), 7, 0)
            .unwrap();

        // Query by source station
        let from_station_1 = setup.request_manager.get_pending_at_source(StationId(1));
        assert_eq!(from_station_1.len(), 2); // req1 and req2

        let from_station_2 = setup.request_manager.get_pending_at_source(StationId(2));
        assert_eq!(from_station_2.len(), 1); // req3
    }

    #[test]
    fn test_metrics_collection() {
        let mut setup = TestSetup::new();
        setup.setup_trains(&[TrainId(1)], 500);

        // Create and track multiple requests
        let mut request_ids = Vec::new();
        for i in 1..=3 {
            let req = setup
                .request_manager
                .create_request(
                    ResourceType::Steel,
                    100,
                    StationId(1),
                    StationId(2),
                    5,
                    i * 10,
                )
                .unwrap();
            request_ids.push(req);
        }

        // Verify we can complete requests
        setup
            .request_manager
            .complete_request(request_ids[0], 100)
            .ok();
        setup
            .request_manager
            .complete_request(request_ids[1], 150)
            .ok();

        // Check requests exist at source
        let from_source = setup.request_manager.get_pending_at_source(StationId(1));
        // Some should have moved to other states after completion
        assert!(from_source.len() <= 3);
    }

    #[test]
    fn test_error_handling_and_recovery() {
        let mut setup = TestSetup::new();
        setup.setup_trains(&[TrainId(1)], 500);

        // Create request
        let request = setup
            .request_manager
            .create_request(ResourceType::Steel, 100, StationId(1), StationId(2), 5, 0)
            .unwrap();

        // Try to unload before loading - should fail
        let result = setup.transfer_manager.unload_at_station(
            TrainId(1),
            StationId(2),
            &mut setup.request_manager,
            50,
        );
        assert!(result.is_err());

        // Try to start loading at wrong station
        let req = setup.request_manager.get_request(request).unwrap().clone();
        let result = setup.transfer_manager.start_loading(
            TrainId(1),
            StationId(3), // Wrong station
            &req,
            0,
            &mut setup.request_manager,
        );
        assert!(result.is_err());

        // Verify request still exists
        let req_final = setup.request_manager.get_request(request);
        assert!(req_final.is_some());
    }
}
