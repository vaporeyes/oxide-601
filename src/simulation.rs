// ABOUTME: Simulation engine for the Minivac 601 computer.
// ABOUTME: Models the electromechanical state, electrical solver, and component ports.

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ComponentPort {
    PowerPlus(usize),         // Positive power terminal index 0..3
    PowerMinus(usize),        // Negative power terminal index 0..3
    Pushbutton(usize, char),   // Button index 0..5, terminal 'X', 'Y', 'Z'
    SlideSwitch(usize, char),  // Switch index 0..5, terminal 'R', 'S', 'T', 'U', 'V', 'W'
    RelayCoil(usize, char),    // Relay index 0..5, terminal 'C', 'E', 'F'
    RelayContact(usize, char), // Relay index 0..5, terminal 'G', 'H', 'J', 'K', 'L', 'N'
    BinaryLight(usize, char),  // Light index 0..5, terminal 'A', 'B'
    DialContact(usize),        // Dial contact D0..D15 (index 0..15)
    DialWiper,                 // Dial wiper D16
    DialMotor(usize),          // Motor terminal 17, 18, 19
    TiePoint(usize, usize),    // Tie-point (1com..6com) index 0..5, hole index 0..3
    MatrixPoint(usize, char),  // Matrix point index 0..10, hole 't' or 'b'
}

impl ComponentPort {
    #[allow(dead_code)]
    pub fn name(&self) -> String {
        match self {
            ComponentPort::PowerPlus(i) => format!("+{i}"),
            ComponentPort::PowerMinus(i) => format!("-{i}"),
            ComponentPort::Pushbutton(i, c) => format!("PB{}_{}", i + 1, c),
            ComponentPort::SlideSwitch(i, c) => format!("SW{}_{}", i + 1, c),
            ComponentPort::RelayCoil(i, c) => format!("RLY{}_coil_{}", i + 1, c),
            ComponentPort::RelayContact(i, c) => format!("RLY{}_contact_{}", i + 1, c),
            ComponentPort::BinaryLight(i, c) => format!("LGT{}_{}", i + 1, c),
            ComponentPort::DialContact(i) => format!("D{i}"),
            ComponentPort::DialWiper => "D16".to_string(),
            ComponentPort::DialMotor(i) => format!("M{i}"),
            ComponentPort::TiePoint(i, h) => format!("COM{}_h{}", i + 1, h),
            ComponentPort::MatrixPoint(i, c) => format!("M{}{}", i + 1, c),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Connection {
    pub from: Uuid,
    pub from_hole: usize,
    pub to: Uuid,
    pub to_hole: usize,
    pub color: [u8; 4], // RGBA in u8
}

pub struct LoadEdge {
    pub node_a: Uuid,
    pub node_b: Uuid,
    pub resistance: f32,
    pub load_type: LoadType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadType {
    Light(usize),
    RelayLight(usize),
    RelayCoil(usize),
    Motor,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct MinivacSimulation {
    pub connections: Vec<Connection>,
    pub buttons_pressed: [bool; 6],
    pub switches_right: [bool; 6],
    pub relay_progress: [f32; 6],
    pub relay_state: [bool; 6],
    pub dial_position: f32, // 0.0 to 16.0
    pub motor_running: bool,
    pub breaker_tripped: bool,
    pub power_on: bool,

    #[serde(skip)]
    pub port_to_uuid: HashMap<ComponentPort, Uuid>,
    #[serde(skip)]
    pub uuid_to_port: HashMap<Uuid, ComponentPort>,
    #[serde(skip)]
    pub port_voltages: HashMap<Uuid, f32>,
    #[serde(skip)]
    pub light_brightness: [f32; 6],
    #[serde(skip)]
    pub relay_light_brightness: [f32; 6],
}

impl Default for MinivacSimulation {
    fn default() -> Self {
        Self::new()
    }
}

impl MinivacSimulation {
    pub fn new() -> Self {
        let mut port_to_uuid = HashMap::new();
        let mut uuid_to_port = HashMap::new();

        let mut add_port = |port: ComponentPort| {
            let id = Uuid::new_v4();
            port_to_uuid.insert(port, id);
            uuid_to_port.insert(id, port);
        };

        // Power terminals
        for i in 0..4 {
            add_port(ComponentPort::PowerPlus(i));
            add_port(ComponentPort::PowerMinus(i));
        }

        // Pushbuttons (1..6)
        for i in 0..6 {
            add_port(ComponentPort::Pushbutton(i, 'X'));
            add_port(ComponentPort::Pushbutton(i, 'Y'));
            add_port(ComponentPort::Pushbutton(i, 'Z'));
        }

        // Slide Switches (1..6)
        for i in 0..6 {
            add_port(ComponentPort::SlideSwitch(i, 'R'));
            add_port(ComponentPort::SlideSwitch(i, 'S'));
            add_port(ComponentPort::SlideSwitch(i, 'T'));
            add_port(ComponentPort::SlideSwitch(i, 'U'));
            add_port(ComponentPort::SlideSwitch(i, 'V'));
            add_port(ComponentPort::SlideSwitch(i, 'W'));
        }

        // Relays (1..6)
        for i in 0..6 {
            add_port(ComponentPort::RelayCoil(i, 'C'));
            add_port(ComponentPort::RelayCoil(i, 'E'));
            add_port(ComponentPort::RelayCoil(i, 'F'));
            add_port(ComponentPort::RelayContact(i, 'G'));
            add_port(ComponentPort::RelayContact(i, 'H'));
            add_port(ComponentPort::RelayContact(i, 'J'));
            add_port(ComponentPort::RelayContact(i, 'K'));
            add_port(ComponentPort::RelayContact(i, 'L'));
            add_port(ComponentPort::RelayContact(i, 'N'));
        }

        // Binary Output Lights (1..6)
        for i in 0..6 {
            add_port(ComponentPort::BinaryLight(i, 'A'));
            add_port(ComponentPort::BinaryLight(i, 'B'));
        }

        // Dial Contacts
        for i in 0..16 {
            add_port(ComponentPort::DialContact(i));
        }
        add_port(ComponentPort::DialWiper);

        // Dial Motor Control
        add_port(ComponentPort::DialMotor(17));
        add_port(ComponentPort::DialMotor(18));
        add_port(ComponentPort::DialMotor(19));

        // Tie points (1com..6com)
        for i in 0..6 {
            for h in 0..4 {
                add_port(ComponentPort::TiePoint(i, h));
            }
        }

        // Matrix (M1..M11)
        for i in 0..11 {
            add_port(ComponentPort::MatrixPoint(i, 't'));
            add_port(ComponentPort::MatrixPoint(i, 'b'));
        }

        Self {
            connections: Vec::new(),
            buttons_pressed: [false; 6],
            switches_right: [false; 6],
            relay_progress: [0.0; 6],
            relay_state: [false; 6],
            dial_position: 0.0,
            motor_running: false,
            breaker_tripped: false,
            power_on: false,
            port_to_uuid,
            uuid_to_port,
            port_voltages: HashMap::new(),
            light_brightness: [0.0; 6],
            relay_light_brightness: [0.0; 6],
        }
    }

    // Restore logical mapping after deserialization
    #[allow(dead_code)]
    pub fn rebuild_mappings(&mut self) {
        let sim = Self::new();
        self.port_to_uuid = sim.port_to_uuid;
        self.uuid_to_port = sim.uuid_to_port;
    }

    pub fn add_connection(&mut self, from: Uuid, from_hole: usize, to: Uuid, to_hole: usize, color: [u8; 4]) {
        if from != to {
            self.connections.push(Connection { from, from_hole, to, to_hole, color });
        }
    }

    pub fn tick(&mut self, dt: f32) {
        // 1. Solve electrical state
        self.solve_electrical();

        // 2. Update relay states based on coil voltage drop
        let relay_latency = 0.05; // 50ms transition delay
        for i in 0..6 {
            let coil_e = self.port_to_uuid[&ComponentPort::RelayCoil(i, 'E')];
            let coil_f = self.port_to_uuid[&ComponentPort::RelayCoil(i, 'F')];
            let v_e = *self.port_voltages.get(&coil_e).unwrap_or(&0.0);
            let v_f = *self.port_voltages.get(&coil_f).unwrap_or(&0.0);
            let coil_voltage_drop = (v_e - v_f).abs();

            if coil_voltage_drop >= 8.0 {
                self.relay_progress[i] = (self.relay_progress[i] + dt / relay_latency).min(1.0);
            } else {
                self.relay_progress[i] = (self.relay_progress[i] - dt / relay_latency).max(0.0);
            }

            if self.relay_progress[i] >= 1.0 {
                self.relay_state[i] = true;
            } else if self.relay_progress[i] <= 0.0 {
                self.relay_state[i] = false;
            }
        }

        // 3. Update dial motor rotation
        if self.motor_running {
            let motor_speed = 3.0; // positions per second
            self.dial_position = (self.dial_position + motor_speed * dt) % 16.0;
        }
    }

    fn solve_electrical(&mut self) {
        self.port_voltages.clear();
        self.light_brightness = [0.0; 6];
        self.relay_light_brightness = [0.0; 6];
        self.motor_running = false;

        if !self.power_on {
            return;
        }

        // Identify all active zero-resistance connections
        let mut adj: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        let add_zero_edge = |a: Uuid, b: Uuid, adj: &mut HashMap<Uuid, Vec<Uuid>>| {
            adj.entry(a).or_default().push(b);
            adj.entry(b).or_default().push(a);
        };

        // User patch connections
        for conn in &self.connections {
            add_zero_edge(conn.from, conn.to, &mut adj);
        }

        // Pushbuttons internal connections
        for i in 0..6 {
            let y = self.port_to_uuid[&ComponentPort::Pushbutton(i, 'Y')];
            if self.buttons_pressed[i] {
                let x = self.port_to_uuid[&ComponentPort::Pushbutton(i, 'X')];
                add_zero_edge(y, x, &mut adj);
            } else {
                let z = self.port_to_uuid[&ComponentPort::Pushbutton(i, 'Z')];
                add_zero_edge(y, z, &mut adj);
            }
        }

        // Slide switches internal connections
        for i in 0..6 {
            let s = self.port_to_uuid[&ComponentPort::SlideSwitch(i, 'S')];
            let v = self.port_to_uuid[&ComponentPort::SlideSwitch(i, 'V')];
            if self.switches_right[i] {
                let t = self.port_to_uuid[&ComponentPort::SlideSwitch(i, 'T')];
                let w = self.port_to_uuid[&ComponentPort::SlideSwitch(i, 'W')];
                add_zero_edge(s, t, &mut adj);
                add_zero_edge(v, w, &mut adj);
            } else {
                let r = self.port_to_uuid[&ComponentPort::SlideSwitch(i, 'R')];
                let u = self.port_to_uuid[&ComponentPort::SlideSwitch(i, 'U')];
                add_zero_edge(s, r, &mut adj);
                add_zero_edge(v, u, &mut adj);
            }
        }

        // Relays internal connections
        for i in 0..6 {
            let h = self.port_to_uuid[&ComponentPort::RelayContact(i, 'H')];
            let l = self.port_to_uuid[&ComponentPort::RelayContact(i, 'L')];

            if self.relay_progress[i] >= 1.0 {
                let g = self.port_to_uuid[&ComponentPort::RelayContact(i, 'G')];
                let k = self.port_to_uuid[&ComponentPort::RelayContact(i, 'K')];
                add_zero_edge(h, g, &mut adj);
                add_zero_edge(l, k, &mut adj);
            } else if self.relay_progress[i] <= 0.0 {
                let j = self.port_to_uuid[&ComponentPort::RelayContact(i, 'J')];
                let n = self.port_to_uuid[&ComponentPort::RelayContact(i, 'N')];
                add_zero_edge(h, j, &mut adj);
                add_zero_edge(l, n, &mut adj);
            }
            // In transition, contacts are break-before-make (open circuit)
        }

        // Rotary Switch wiper alignment contact
        let dial_idx = self.dial_position.floor() as usize;
        let frac = self.dial_position - dial_idx as f32;
        let wiper_contact = if frac <= 0.2 || frac >= 0.8 {
            let contact_idx = if frac >= 0.8 { (dial_idx + 1) % 16 } else { dial_idx };
            Some(self.port_to_uuid[&ComponentPort::DialContact(contact_idx)])
        } else {
            None
        };

        if let Some(contact_uuid) = wiper_contact {
            let wiper_uuid = self.port_to_uuid[&ComponentPort::DialWiper];
            add_zero_edge(wiper_uuid, contact_uuid, &mut adj);
        }

        // Matrix top and bottom terminals are connected together internally
        for i in 0..11 {
            let t = self.port_to_uuid[&ComponentPort::MatrixPoint(i, 't')];
            let b = self.port_to_uuid[&ComponentPort::MatrixPoint(i, 'b')];
            add_zero_edge(t, b, &mut adj);
        }

        // Tie-points internally connected
        for i in 0..6 {
            let h0 = self.port_to_uuid[&ComponentPort::TiePoint(i, 0)];
            for h in 1..4 {
                let h_u = self.port_to_uuid[&ComponentPort::TiePoint(i, h)];
                add_zero_edge(h0, h_u, &mut adj);
            }
        }

        // Group ports into equivalence classes (nets)
        let mut visited = HashSet::new();
        let mut nets: Vec<Vec<Uuid>> = Vec::new();
        let mut uuid_to_net_idx: HashMap<Uuid, usize> = HashMap::new();

        for &uuid in self.uuid_to_port.keys() {
            if !visited.contains(&uuid) {
                let mut net = Vec::new();
                let mut queue = vec![uuid];
                visited.insert(uuid);

                while let Some(curr) = queue.pop() {
                    net.push(curr);
                    uuid_to_net_idx.insert(curr, nets.len());
                    if let Some(neighbors) = adj.get(&curr) {
                        for &neigh in neighbors {
                            if !visited.contains(&neigh) {
                                visited.insert(neigh);
                                queue.push(neigh);
                            }
                        }
                    }
                }
                nets.push(net);
            }
        }

        // Check for short circuits
        if !self.breaker_tripped {
            for net in &nets {
                let mut has_plus = false;
                let mut has_minus = false;
                for &uuid in net {
                    if let Some(port) = self.uuid_to_port.get(&uuid) {
                        match port {
                            ComponentPort::PowerPlus(_) => has_plus = true,
                            ComponentPort::PowerMinus(_) => has_minus = true,
                            _ => {}
                        }
                    }
                }
                if has_plus && has_minus {
                    self.breaker_tripped = true;
                    return;
                }
            }
        }

        if self.breaker_tripped {
            return;
        }

        // Define loads
        let mut loads = Vec::new();

        // 6 Binary Output Lights
        for i in 0..6 {
            loads.push(LoadEdge {
                node_a: self.port_to_uuid[&ComponentPort::BinaryLight(i, 'A')],
                node_b: self.port_to_uuid[&ComponentPort::BinaryLight(i, 'B')],
                resistance: 100.0,
                load_type: LoadType::Light(i),
            });
        }

        // 6 Relays indicator lights and coils
        for i in 0..6 {
            // Light between C and E
            loads.push(LoadEdge {
                node_a: self.port_to_uuid[&ComponentPort::RelayCoil(i, 'C')],
                node_b: self.port_to_uuid[&ComponentPort::RelayCoil(i, 'E')],
                resistance: 100.0,
                load_type: LoadType::RelayLight(i),
            });
            // Coil between E and F
            loads.push(LoadEdge {
                node_a: self.port_to_uuid[&ComponentPort::RelayCoil(i, 'E')],
                node_b: self.port_to_uuid[&ComponentPort::RelayCoil(i, 'F')],
                resistance: 200.0,
                load_type: LoadType::RelayCoil(i),
            });
        }

        // Motor between 17 and 18
        loads.push(LoadEdge {
            node_a: self.port_to_uuid[&ComponentPort::DialMotor(17)],
            node_b: self.port_to_uuid[&ComponentPort::DialMotor(18)],
            resistance: 150.0,
            load_type: LoadType::Motor,
        });

        // Setup linear equations for Net Voltages
        // Net voltage:
        // - Net containing any PowerPlus is fixed at 12.0V
        // - Net containing any PowerMinus is fixed at 0.0V
        // - Other nets are solved iteratively
        let mut net_fixed_val: Vec<Option<f32>> = vec![None; nets.len()];
        for (idx, net) in nets.iter().enumerate() {
            let mut is_plus = false;
            let mut is_minus = false;
            for &uuid in net {
                if let Some(port) = self.uuid_to_port.get(&uuid) {
                    match port {
                        ComponentPort::PowerPlus(_) => is_plus = true,
                        ComponentPort::PowerMinus(_) => is_minus = true,
                        _ => {}
                    }
                }
            }
            if is_plus {
                net_fixed_val[idx] = Some(12.0);
            } else if is_minus {
                net_fixed_val[idx] = Some(0.0);
            }
        }

        // Solve net voltages using Jacobi iteration
        let mut net_voltages = vec![0.0; nets.len()];
        // Initialize fixed voltages
        for idx in 0..nets.len() {
            if let Some(v) = net_fixed_val[idx] {
                net_voltages[idx] = v;
            }
        }

        // Compile connections between nets through loads
        struct NetLoad {
            target_net: usize,
            cond: f32, // Conductance = 1 / R
        }
        let mut net_loads: Vec<Vec<NetLoad>> = (0..nets.len()).map(|_| Vec::new()).collect();
        for load in &loads {
            let net_a = uuid_to_net_idx[&load.node_a];
            let net_b = uuid_to_net_idx[&load.node_b];
            if net_a != net_b {
                let cond = 1.0 / load.resistance;
                net_loads[net_a].push(NetLoad { target_net: net_b, cond });
                net_loads[net_b].push(NetLoad { target_net: net_a, cond });
            }
        }

        // Jacobi Iterations
        for _ in 0..100 {
            let mut next_voltages = net_voltages.clone();
            for idx in 0..nets.len() {
                if net_fixed_val[idx].is_some() {
                    continue;
                }
                let mut sum_num = 0.0;
                let mut sum_den = 0.0;
                for load in &net_loads[idx] {
                    sum_num += net_voltages[load.target_net] * load.cond;
                    sum_den += load.cond;
                }
                if sum_den > 0.0 {
                    next_voltages[idx] = sum_num / sum_den;
                } else {
                    next_voltages[idx] = 0.0;
                }
            }
            net_voltages = next_voltages;
        }

        // Propagate net voltages to individual ports
        for (idx, net) in nets.iter().enumerate() {
            let v = net_voltages[idx];
            for &uuid in net {
                self.port_voltages.insert(uuid, v);
            }
        }

        // Calculate load state outputs
        for load in &loads {
            let v_a = *self.port_voltages.get(&load.node_a).unwrap_or(&0.0);
            let v_b = *self.port_voltages.get(&load.node_b).unwrap_or(&0.0);
            let drop = (v_a - v_b).abs();

            match load.load_type {
                LoadType::Light(i) => {
                    self.light_brightness[i] = (drop / 12.0).powi(2).min(1.0);
                }
                LoadType::RelayLight(i) => {
                    self.relay_light_brightness[i] = (drop / 12.0).powi(2).min(1.0);
                }
                LoadType::RelayCoil(_) => {}
                LoadType::Motor => {
                    // Check if STOP terminals (18 and 19) are shorted
                    let stop_18 = self.port_to_uuid[&ComponentPort::DialMotor(18)];
                    let stop_19 = self.port_to_uuid[&ComponentPort::DialMotor(19)];
                    let stop_shorted = uuid_to_net_idx[&stop_18] == uuid_to_net_idx[&stop_19];

                    if drop >= 8.0 && !stop_shorted {
                        self.motor_running = true;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_circuit() {
        let mut sim = MinivacSimulation::new();
        sim.power_on = true;

        let plus_0 = sim.port_to_uuid[&ComponentPort::PowerPlus(0)];
        let minus_0 = sim.port_to_uuid[&ComponentPort::PowerMinus(0)];
        let light_a = sim.port_to_uuid[&ComponentPort::BinaryLight(0, 'A')];
        let light_b = sim.port_to_uuid[&ComponentPort::BinaryLight(0, 'B')];

        // Wire +0 -> light_A, light_B -> -0
        sim.add_connection(plus_0, 0, light_a, 0, [255, 0, 0, 255]);
        sim.add_connection(light_b, 0, minus_0, 0, [255, 0, 0, 255]);

        sim.tick(0.01);

        assert!(!sim.breaker_tripped);
        assert!(sim.light_brightness[0] > 0.9);
    }

    #[test]
    fn test_short_circuit() {
        let mut sim = MinivacSimulation::new();
        sim.power_on = true;

        let plus_0 = sim.port_to_uuid[&ComponentPort::PowerPlus(0)];
        let minus_0 = sim.port_to_uuid[&ComponentPort::PowerMinus(0)];

        // Wire +0 -> -0 directly
        sim.add_connection(plus_0, 0, minus_0, 0, [255, 0, 0, 255]);

        sim.tick(0.01);

        assert!(sim.breaker_tripped);
    }

    #[test]
    fn test_pushbutton_state() {
        let mut sim = MinivacSimulation::new();
        sim.power_on = true;

        let plus_0 = sim.port_to_uuid[&ComponentPort::PowerPlus(0)];
        let minus_0 = sim.port_to_uuid[&ComponentPort::PowerMinus(0)];
        let pb_y = sim.port_to_uuid[&ComponentPort::Pushbutton(0, 'Y')];
        let pb_x = sim.port_to_uuid[&ComponentPort::Pushbutton(0, 'X')];
        let light_a = sim.port_to_uuid[&ComponentPort::BinaryLight(0, 'A')];
        let light_b = sim.port_to_uuid[&ComponentPort::BinaryLight(0, 'B')];

        // Wire +0 -> pb_y
        sim.add_connection(plus_0, 0, pb_y, 0, [255, 0, 0, 255]);
        // Wire pb_x -> light_a
        sim.add_connection(pb_x, 0, light_a, 0, [255, 0, 0, 255]);
        // Wire light_b -> -0
        sim.add_connection(light_b, 0, minus_0, 0, [255, 0, 0, 255]);

        // Unpressed
        sim.tick(0.01);
        assert!(sim.light_brightness[0] < 0.1);

        // Pressed
        sim.buttons_pressed[0] = true;
        sim.tick(0.01);
        assert!(sim.light_brightness[0] > 0.9);
    }

    #[test]
    fn test_relay_latching_and_latency() {
        let mut sim = MinivacSimulation::new();
        sim.power_on = true;

        let plus_0 = sim.port_to_uuid[&ComponentPort::PowerPlus(0)];
        let minus_0 = sim.port_to_uuid[&ComponentPort::PowerMinus(0)];
        let coil_c = sim.port_to_uuid[&ComponentPort::RelayCoil(0, 'C')];
        let coil_f = sim.port_to_uuid[&ComponentPort::RelayCoil(0, 'F')];

        // Wire +0 -> coil_c, coil_f -> -0
        sim.add_connection(plus_0, 0, coil_c, 0, [255, 0, 0, 255]);
        sim.add_connection(coil_f, 0, minus_0, 0, [255, 0, 0, 255]);

        // Initially relay progress should be 0.0
        assert_eq!(sim.relay_progress[0], 0.0);
        assert!(!sim.relay_state[0]);

        // First tick
        sim.tick(0.02);
        assert!(sim.relay_progress[0] > 0.0);
        assert!(sim.relay_progress[0] < 1.0);
        assert!(!sim.relay_state[0]);

        // Tick enough times to complete transition (mechanical latency is 50ms)
        for _ in 0..10 {
            sim.tick(0.01);
        }
        assert_eq!(sim.relay_progress[0], 1.0);
        assert!(sim.relay_state[0]);
    }

    #[test]
    fn test_motor_run_stop() {
        let mut sim = MinivacSimulation::new();
        sim.power_on = true;

        let plus_0 = sim.port_to_uuid[&ComponentPort::PowerPlus(0)];
        let minus_0 = sim.port_to_uuid[&ComponentPort::PowerMinus(0)];
        let m17 = sim.port_to_uuid[&ComponentPort::DialMotor(17)];
        let m18 = sim.port_to_uuid[&ComponentPort::DialMotor(18)];
        let m19 = sim.port_to_uuid[&ComponentPort::DialMotor(19)];

        // Connect +0 -> M17, M18 -> -0. Motor should run.
        sim.add_connection(plus_0, 0, m17, 0, [255, 0, 0, 255]);
        sim.add_connection(m18, 0, minus_0, 0, [255, 0, 0, 255]);

        sim.tick(0.01);
        assert!(sim.motor_running);

        // Short M18 -> M19. Motor should stop due to electromechanical brake.
        sim.add_connection(m18, 1, m19, 0, [255, 0, 0, 255]);
        sim.tick(0.01);
        assert!(!sim.motor_running);
    }
}
