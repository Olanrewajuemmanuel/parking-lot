use parking_lot::{
    Account, Parkable, ParkingFloor, ParkingLot, ParkingSpot, SpotType, User, Vehicle, VehicleType,
};

fn main() {
    println!("🅿️ Parking Lot Project Demo");
    println!("Low-Level Design for Interview purposes");

    let mut parking_lot = ParkingLot::new(
        "Park-Wella Parking Hub".into(),
        "Lagos, Nigeria".into(),
        "1234".into(),
    );

    // ===============
    // For compactness sake, each floor will initialize ten spots with the regular type. However,
    // an API, `add_spot()` will be exposed for each floor to add more spots as needed. This is illustrated below.
    // ===============

    for i in 1..=5 {
        parking_lot.add_floor(ParkingFloor::new(i));
    }

    let mut floor1 = parking_lot.get_floor_by_id(1).unwrap();

    for _ in 0..5 {
        floor1.add_spot(ParkingSpot::new(
            true,
            SpotType::Large, // Assume big trucks should be on base floor
        ));
    }

    parking_lot.display_info();

    let vehicle1 = Vehicle::new(VehicleType::Motor, "Toyota".into(), "ABC123".into());
    let vehicle2 = Vehicle::new(VehicleType::Truck, "Mac".into(), "XYZ789".into());
    let vehicle3 = Vehicle::new(VehicleType::Bike, "Suzuki".into(), "DEF456".into());

    {
        // Book a Parking spot
        let mut user1 = User::new("Larry".into(), "123".into());

        user1.register_vehicle(vehicle1.clone());
        user1.register_vehicle(vehicle2.clone());
        user1.register_vehicle(vehicle3.clone());
        

        match parking_lot.park_vehicle(vehicle1) {
            Ok(ticket) => println!("Motor parked with ticket id: {}", ticket.ticket_id),
            Err(e) => eprintln!("An error occured while getting your parking ticket {e}"),
        };
        match parking_lot.park_vehicle(vehicle2) {
            Ok(ticket) => println!("Motor parked with ticket id: {}", ticket.ticket_id),
            Err(e) => eprintln!("An error occured while getting your parking ticket {e}"),
        };
        let mut test_ticket = String::new();


        match parking_lot.park_vehicle(vehicle3) {
            Ok(ticket) => {
                println!("Motor parked with ticket id: {}", ticket.ticket_id);
                test_ticket = ticket.ticket_id;
            }
            Err(e) => eprintln!("An error occured while getting your parking ticket {e}"),
        };

        parking_lot.display_info();

        match parking_lot.unpark_vehicle(test_ticket.to_string()) {
            Ok(charge) => println!(
                "Vehicle successfully unparked. Grand total: {}, chargeback: {}",
                charge.total, charge.chargeback
            ),
            Err(e) => eprintln!("An error occured while unparking: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parking_lot_initializes_with_required_number_of_floors() {
        let mut parking_lot = ParkingLot::new(
            "Park-Wella Parking Hub".into(),
            "Lagos, Nigeria".into(),
            "1234".into(),
        );

        for i in 1..=5 {
            parking_lot.add_floor(ParkingFloor::new(i));
        }

        assert_eq!(parking_lot.display_info().num_floors(), 5);
    }
}
