//! Console simulation of the wired two-float-switch controller.
//! This is a host-side simulator; replace the simulated inputs and pump
//! hardware with ESP32 GPIO and a properly isolated pump-driver interface
//! when moving to the embedded target.

use std::io::{self, Write};
use std::time::Instant;

use crate::enum_and_transition::{PumpEvent, PumpState, TankLevel};
use crate::pump_controller::{apply_output, FakePumpHardware, PumpController};
use crate::tank_sensors::{interpret_tank_level, FloatInputs};

pub fn run() {
    println!("============================================================");
    println!(" WATER TANK AUTOMATOR - WIRED FLOAT-SWITCH SIMULATION");
    println!("============================================================");
    println!("Commands:");
    println!("  l -> LOW switch active");
    println!("  n -> both switches inactive (NORMAL)");
    println!("  f -> FULL switch active");
    println!("  e -> both switches active (sensor fault)");
    println!("  x -> reset controller");
    println!("  s -> show status");
    println!("  q -> quit");
    println!();

    let start = Instant::now();
    let mut controller = PumpController::new();
    let mut pump = FakePumpHardware::new();
    let mut inputs = FloatInputs {
        low_active: false,
        full_active: false,
    };
    let mut level = interpret_tank_level(inputs);

    loop {
        print!("Command [l/n/f/e/x/s/q]: ");
        if io::stdout().flush().is_err() {
            return;
        }

        let mut command = String::new();
        match io::stdin().read_line(&mut command) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }

        let current_ms = start.elapsed().as_millis() as u32;
        match command.trim().to_lowercase().as_str() {
            "l" => {
                inputs = FloatInputs { low_active: true, full_active: false };
                level = interpret_tank_level(inputs);
                update_controller(level, current_ms, &mut controller, &mut pump);
            }
            "n" => {
                inputs = FloatInputs { low_active: false, full_active: false };
                level = interpret_tank_level(inputs);
                update_controller(level, current_ms, &mut controller, &mut pump);
            }
            "f" => {
                inputs = FloatInputs { low_active: false, full_active: true };
                level = interpret_tank_level(inputs);
                update_controller(level, current_ms, &mut controller, &mut pump);
            }
            "e" => {
                inputs = FloatInputs { low_active: true, full_active: true };
                level = interpret_tank_level(inputs);
                update_controller(level, current_ms, &mut controller, &mut pump);
            }
            "x" => {
                let output = controller.handle_event(PumpEvent::Reset, current_ms);
                apply_output(output, &mut pump);
                println!("Controller reset; pump is OFF.");
            }
            "s" => print_status(level, &controller, &pump),
            "q" => {
                println!("Simulation stopped.");
                return;
            }
            "" => {}
            other => println!("Unknown command: {other}"),
        }

        // Enforce the maximum pump-run time on every loop iteration.
        let output = controller.handle_max_run_timeout(current_ms);
        apply_output(output, &mut pump);
    }
}

fn update_controller(
    level: TankLevel,
    current_ms: u32,
    controller: &mut PumpController,
    pump: &mut FakePumpHardware,
) {
    let output = controller.handle_event(
        PumpEvent::TankLevel { level_of_tank: level },
        current_ms,
    );
    apply_output(output, pump);
    println!("Tank level: {level:?}");
    println!("Pump: {:?}", controller.state());
    if let Some(reason) = controller.faultreason() {
        println!("Fault: {reason:?}");
    }
}

fn print_status(level: TankLevel, controller: &PumpController, pump: &FakePumpHardware) {
    println!("Tank level: {level:?}");
    println!("Controller state: {:?}", controller.state());
    println!("Pump output: {}", if pump.pump_is_on { "ON" } else { "OFF" });
    println!("Fault: {:?}", controller.faultreason());
    if controller.state() == PumpState::Fault {
        println!("Fault is latched; use 'x' to reset.");
    }
}
