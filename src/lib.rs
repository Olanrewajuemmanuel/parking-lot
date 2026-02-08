use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};

#[derive(Debug)]
pub enum SpotType {
    Large,
    Regular,
    XLarge,
    Handicapped,
}

#[derive(Debug, Clone)]
pub enum VehicleType {
    Motor,
    Truck,
    Bike,
}

#[derive(Debug, Clone)]
pub enum PaymentStatus {
    Succeeded,
    Failed,
    Pending,
}

// === PARKING LOT ===

#[derive(Debug)]
pub struct ParkingLot {
    name: String,
    address: String,
    uid: String,
    floors: Arc<Mutex<HashMap<u32, ParkingFloor>>>,
    active_tickets: Arc<Mutex<HashMap<String, ParkingTicket>>>,
}

pub struct ParkingLotDisplayBoard {
    uid: String,
    num_floors: u32,
    num_empty_spots: u32,
    num_parked_vehicles: u32,
}

#[derive(Debug, Clone)]
pub struct ParkingTicket {
    pub ticket_id: String,
    pub vehicle: Vehicle,
    pub spot_id: String,
    pub entry_time: DateTime<Utc>,
    pub exit_time: Option<DateTime<Utc>>,
    pub payment_status: PaymentStatus,
}

impl ParkingTicket {
    pub fn new(ticket_id: String, vehicle: Vehicle, spot_id: String) -> Self {
        Self {
            ticket_id,
            vehicle,
            spot_id,
            entry_time: Utc::now(),
            exit_time: None,
            payment_status: PaymentStatus::Pending,
        }
    }
}

pub struct ParkingCharge {
    pub total: f32,
    pub chargeback: f32,
}

impl ParkingLot {
    pub fn new(name: String, address: String, uid: String) -> Self {
        Self {
            name,
            address,
            uid,
            floors: Arc::new(Mutex::new(HashMap::new())),
            active_tickets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn generate_ticket_id(&self) -> String {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        format!("TKT_{}", COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }

    pub fn add_floor(&mut self, floor: ParkingFloor) {
        let mut floors = self.floors.lock().unwrap();
        floors.insert(floor.id, floor);
    }

    pub fn get_floor_by_id(&self, id: u32) -> Option<ParkingFloor> {
        let floors = self.floors.lock().unwrap();
        floors.get(&id).cloned()
    }

    pub fn display_info(&self) -> ParkingLotDisplayBoard {
        let floors = self.floors.lock().unwrap();
        ParkingLotDisplayBoard {
            uid: self.uid.clone(),
            num_floors: floors.len() as u32,
            num_empty_spots: floors
                .values()
                .map(|f| f.spots.lock().unwrap().len() as u32)
                .sum(),
            num_parked_vehicles: floors
                .values()
                .map(|f| {
                    f.spots
                        .lock()
                        .unwrap()
                        .values()
                        .filter(|s| !s.is_free)
                        .count() as u32
                })
                .sum(),
        }
    }
}

pub trait Parkable {
    fn park_vehicle(&self, vehicle: Vehicle) -> Result<ParkingTicket, String>;
    fn unpark_vehicle(&self, ticket_id: String) -> Result<ParkingCharge, String>;
}

impl Parkable for ParkingLot {
    fn park_vehicle(&self, vehicle: Vehicle) -> Result<ParkingTicket, String> {
        let available_spot = {
            let floors = self.floors.lock().unwrap();
            floors.values().find_map(|floor| {
                floor.find_available_spot(vehicle.vehicle_type.clone())
            })
        }.ok_or("No available spots")?;

        let (floor_number, spot_id) = available_spot;

        // Assign vehicle to spot
        let mut floors = self.floors.lock().unwrap();
        let floor = floors.get_mut(&floor_number).unwrap();
        let mut spots = floor.spots.lock().unwrap();
        let spot = spots.get_mut(&spot_id).unwrap();
        spot.assign_vehicle(vehicle.clone())?;

        // Create ticket
        let ticket_id = self.generate_ticket_id();
        let ticket = ParkingTicket::new(ticket_id, vehicle, spot_id);

        // Store active ticket
        let mut tickets = self.active_tickets.lock().unwrap();
        let ticket_clone = ticket.clone();
        tickets.insert(ticket.ticket_id.clone(), ticket);

        println!("Vehicle parked successfully. Ticket ID: {}", ticket_clone.ticket_id);
        Ok(ticket_clone)
    }

    fn unpark_vehicle(&self, ticket_id: String) -> Result<ParkingCharge, String> {
        // Find and remove ticket
        let mut tickets = self.active_tickets.lock().unwrap();
        let mut ticket = tickets.remove(&ticket_id).ok_or("Invalid ticket ID")?;
        
        // Calculate parking duration and charge
        let now = Utc::now();
        let duration = now.signed_duration_since(ticket.entry_time);
        let hours = duration.num_hours() as f32;
        let rate = 10.0; // $10 per hour
        let total = hours * rate;
        
        // Free the parking spot
        let mut floors = self.floors.lock().unwrap();
        for floor in floors.values_mut() {
            let mut spots = floor.spots.lock().unwrap();
            if let Some(spot) = spots.get_mut(&ticket.spot_id) {
                spot.remove_vehicle();
                break;
            }
        }
        
        // Update ticket with exit time
        ticket.exit_time = Some(now);
        ticket.payment_status = PaymentStatus::Succeeded;
        
        // Return ticket to active_tickets for record keeping
        tickets.insert(ticket_id.clone(), ticket);
        
        let charge = ParkingCharge {
            total,
            chargeback: 0.0,
        };
        
        println!("Vehicle unparked successfully. Total charge: ${:.2}", charge.total);
        Ok(charge)
    }
}

impl ParkingLotDisplayBoard {
    pub fn num_floors(&self) -> u32 {
        self.num_floors
    }

    pub fn num_empty_spots(&self) -> u32 {
        self.num_empty_spots
    }
    
    pub fn num_parked_vehicles(&self) -> u32 {
        self.num_parked_vehicles
    }

}

// === PARKING FLOOR ===
#[derive(Debug, Clone)]
pub struct ParkingFloor {
    id: u32,
    spots: Arc<Mutex<HashMap<String, ParkingSpot>>>,
}

impl ParkingFloor {
    pub fn new(id: u32) -> Self {
        let mut floor = Self {
            id,
            spots: Arc::new(Mutex::new(HashMap::new())),
        };
        floor.initialize_spots();
        floor
    }

    fn initialize_spots(&mut self) {
        // Initialize 10 regular spots by default
        for i in 0..10 {
            self.spots.lock().unwrap().insert(
                format!("spot_{}", i),
                ParkingSpot::new(true, SpotType::Regular),
            );
        }
    }

    pub fn add_spot(&mut self, spot: ParkingSpot) {
        self.spots.lock().unwrap().insert(spot.id.clone(), spot);
    }

    pub fn find_available_spot(&self, vehicle_type: VehicleType) -> Option<(u32, String)> {
        let spots = self.spots.lock().unwrap();
        for (spot_id, spot) in spots.iter() {
            if spot.is_free && spot.is_compatible(&vehicle_type) {
                return Some((self.id, spot_id.clone()));
            }
        }
        None
    }
}

// ===PARKING SPOT ===
#[derive(Debug)]
pub struct ParkingSpot {
    id: String,
    is_free: bool,
    spot_type: SpotType,
    vehicle: Option<Vehicle>,
}

impl ParkingSpot {
    pub fn new(is_free: bool, spot_type: SpotType) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        Self {
            id: format!("spot_{}", COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)),
            is_free,
            spot_type,
            vehicle: None,
        }
    }

    pub fn assign_vehicle(&mut self, vehicle: Vehicle) -> Result<(), String> {
        if !self.is_free {
            return Err("Spot is already occupied".to_string());
        }
        
        if !self.is_compatible(&vehicle.vehicle_type) {
            return Err("Vehicle type not compatible with spot type".to_string());
        }
        
        self.vehicle = Some(vehicle);
        self.is_free = false;
        Ok(())
    }

    pub fn remove_vehicle(&mut self) {
        self.vehicle = None;
        self.is_free = true;
    }

    pub fn is_compatible(&self, vehicle_type: &VehicleType) -> bool {
        match (vehicle_type, &self.spot_type) {
            (VehicleType::Motor, SpotType::Regular) => true,
            (VehicleType::Motor, SpotType::Large) => true,
            (VehicleType::Motor, SpotType::XLarge) => true,
            (VehicleType::Bike, SpotType::Regular) => true,
            (VehicleType::Truck, SpotType::Large) => true,
            (VehicleType::Truck, SpotType::XLarge) => true,
            (VehicleType::Motor, SpotType::Handicapped) => false,
            (VehicleType::Truck, SpotType::Regular) => false,
            (VehicleType::Truck, SpotType::Handicapped) => false,
            (VehicleType::Bike, SpotType::Large) => true,
            (VehicleType::Bike, SpotType::XLarge) => true,
            (VehicleType::Bike, SpotType::Handicapped) => false,
        }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug, Clone)]

pub struct Vehicle {
    vehicle_type: VehicleType,
    model: String,
    license_plate: String,
}

impl Vehicle {
    pub fn new(vehicle_type: VehicleType, model: String, license_plate: String) -> Self {
        Self {
            vehicle_type,
            model,
            license_plate,
        }
    }
}

// === ACCOUNT ===
pub trait Account {
    fn register_vehicle(&mut self, vehicle: Vehicle) -> String;
    fn remove_vehicle(&mut self, vehicle: Vehicle);
    fn get_vehicle_by_id(&self, vehicle_id: String) -> Option<&Vehicle>;
}

#[derive(Debug)]

pub struct User {
    name: String,
    phone: String,
    vehicles: HashMap<String, Vehicle>,
}

impl User {
    pub fn new(name: String, phone: String) -> Self {
        Self {
            name,
            phone,
            vehicles: HashMap::new(),
        }
    }
}

impl Account for User {
    fn register_vehicle(&mut self, vehicle: Vehicle) -> String {
        let vehicle_id = format!("veh_{}", self.vehicles.len() + 1);
        self.vehicles.insert(vehicle_id.clone(), vehicle);
        vehicle_id
    }

    fn remove_vehicle(&mut self, vehicle: Vehicle) {
        self.vehicles.remove(&vehicle.license_plate);
    }

    fn get_vehicle_by_id(&self, vehicle_id: String) -> Option<&Vehicle> {
        self.vehicles.get(&vehicle_id)
    }
}
