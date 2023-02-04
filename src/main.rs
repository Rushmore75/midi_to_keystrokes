use std::collections::VecDeque;
use std::io::{stdin, stdout, Write};
use std::error::Error;
use std::sync::{RwLock, Arc};
use std::time::{Instant, Duration};

use enigo::{Enigo, KeyboardControllable, Key, MouseControllable};
use midir::{MidiInput, Ignore, MidiInputPort, MidiInputConnection};

fn main() {  
    // Create queue for the key strokes
    let queue = Arc::new(RwLock::new(VecDeque::new()));
    // This value just needs to not get droped
    let _conn_in = read_midi(queue.clone()).unwrap();

    let mut en = Enigo::new();
    loop {
        match queue.try_read() {
            Ok(read_queue) => {
                match read_queue.front() {
                    Some(stroke) => {
                        if Instant::now().duration_since(stroke.init) > Duration::new(0, stroke.velocity as u32 * 2 * 1000000 /* conversion ns -> ms */) {
                            println!("Key up: {:?}", stroke.key);
                            en.key_up(stroke.key);
                            // make the lock avaliable
                            drop(read_queue);
                            queue.write().unwrap().pop_front();
                        }
                    }
                    None => {},
                };
            }
            Err(_) => print!("."),
        };
    }
}

fn get_midi_port() -> Result<(MidiInput, MidiInputPort), Box<dyn Error>>  {
    // This code is pretty much 100% from https://docs.rs/crate/midir/latest/source/examples/test_read_input.rs
    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);
    
    // Get an input port (read from console if multiple are available)
    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => return Err("no input port found".into()),
        1 => {
            println!("Choosing the only available input port: {}", midi_in.port_name(&in_ports[0]).unwrap());
            &in_ports[0]
        },
        _ => {
            println!("Available input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                println!("{}: {}", i, midi_in.port_name(p).unwrap());
            }
            print!("Please select input port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            in_ports.get(input.trim().parse::<usize>()?).ok_or("invalid input port selected")?
        }
    };
    Ok((midi_in, in_port.clone())) 
}

fn read_midi(queue: Arc<RwLock<VecDeque<Stroke>>>) -> Result<MidiInputConnection<()>, Box<dyn Error>> {
    // get the port (some user input)
    println!("\nOpening connection");
    let (midi_input, midi_port) = get_midi_port()?;

    // set up key stroke generator
    let mut en = Enigo::new();
    let mut midi_to_keys: [Key; 100] = [Key::Home; 100];

    midi_to_keys[10] = Key::Space;
    midi_to_keys[25] = Key::Layout('w');
    midi_to_keys[35] = Key::Layout('a');
    midi_to_keys[40] = Key::Layout('s');
    midi_to_keys[45] = Key::Layout('d');

    println!("starting...");
    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let conn_in = midi_input.connect(&midi_port, "midir-read-input", move |_, data, _| {
        print!("Note: {}", data[1]);
        if data.len() == 3 {
            // check the midi code, 144 is down
            // while 128(?) is key up we are using a percussion midi instrument, so we don't really
            // deal with length. For a piano or something it might be realivent.
            if data[0] == 144 {
                // mouse stuff
                if data[1] == 30 {
                    println!("moved mosue");
                    en.mouse_move_relative(30, 0);
                    return;
                }
               
                if data[1] == 20 {
                    println!("moved mosue");
                    en.mouse_move_relative(-30, 0);
                    return;
                }

                if data[1] == 15 {
                    println!("lmb");
                    en.mouse_click(enigo::MouseButton::Left);
                }

                // actual key presses
                let key = midi_to_keys[data[1] as usize];
                en.key_down(key);    

                println!("Key down: {:?}", key);
                match queue.write() {
                    Ok(mut x) => {
                        x.push_back(Stroke { init: Instant::now(), velocity: data[2], key, });
                    }, 
                    Err(_) => todo!(),
                }
            } else {
                // println!("Fn code? {:?} | Note {} | Velocity {}", data[0], data[1], data[2]);
            }
        } else {
            println!("{:?}", data);
        }
    }, ())?;
    Ok(conn_in) 
}

#[derive(Clone, Copy)]
struct Stroke {
    init: Instant,
    velocity: u8,
    key: Key,
}
