
#[cfg(test)]
mod tests {
    use super::*;
    use exchange::util::Price;

    #[test]
    fn test_session_data_metrics() {
        let mut session = SessionData::new(1000);
        
        // Simulating volume at prices:
        // 100.0: 10
        // 101.0: 50 (POC)
        // 102.0: 20
        // 103.0: 5
        // Total: 85
        // VA (70%): 59.5
        
        session.add_volume(Price::from_f32(100.0), 10.0);
        session.add_volume(Price::from_f32(101.0), 50.0);
        session.add_volume(Price::from_f32(102.0), 20.0);
        session.add_volume(Price::from_f32(103.0), 5.0);
        
        // Calculate metrics
        session.calculate_metrics(70.0);
        
        // Assert POC
        assert_eq!(session.poc, Some(Price::from_f32(101.0)));
        
        // Assert Value Area
        // POC (50) is < 59.5. 
        // Next highest neighbors: 102 (20) vs 100 (10). Should pick 102.
        // Current vol: 50 + 20 = 70. > 59.5. Done.
        // Bounds should be [101, 102] OR [101, 102] depending on inclusion. 
        // Actually, logic starts at POC and expands.
        // L=P, R=P. 
        // Check L-1 (100, vol 10) vs R+1 (102, vol 20). 20 > 10.
        // Expand R to 102. Vol = 50 + 20 = 70.
        // 70 >= 59.5 target. Stop.
        // VAL = 101.0, VAH = 102.0.
        
        assert_eq!(session.val, Some(Price::from_f32(101.0)));
        assert_eq!(session.vah, Some(Price::from_f32(102.0)));
    }
}
