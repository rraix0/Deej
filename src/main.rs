use std::time::Duration;
use std::io::Read;
use serialport::SerialPort;
use tokio::time::sleep;
use std::fs;
use serde::Deserialize;
use std::process::Command;
use std::sync::{Arc, Mutex};

#[derive(Deserialize, Debug)]
struct Config {
    port: String,
    baud_rate: String,
    count: u8,
    apps: Vec<Vec<String>>, // <-- zamiast struct, mamy tablicę tablic
    reverse: bool,
    read_delay: u64,
}

#[derive(Deserialize, Debug, Clone)]
struct App {
    id: String,
    name: String,
}

//------------------- ARDUINO READER ------------------
async fn read_values_from_serial(port_name: &str, baud_rate: u32, config: &Config) -> Result<(), String> {
    let mut port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(1000))
        .open()
        .map_err(|e| e.to_string())?;

    println!("Connected to serial port {}", port_name);

    let mut buffer: Vec<u8> = vec![0; 32];
    let mut read_buffer = String::new();

    //previous value stores last iterated value, to detect changes
    let mut previous_values: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));

    loop {
        match port.read(buffer.as_mut_slice()) {
            Ok(t) => {
                let data = String::from_utf8_lossy(&buffer[..t]);
                read_buffer.push_str(&data);

                while let Some(pos) = read_buffer.find('\n') {
                    let line = read_buffer[..pos].trim();

                    let values: Vec<i32> = line
                        .split("|")
                        .filter_map(|v| v.parse::<i32>().ok())
                        .collect();

                    //Check if all apps from config are used
                    if values.len() == config.count as usize {



                        let mut values_lock = previous_values.lock().unwrap();


                        if values_lock.is_empty() {
                            *values_lock = values.clone();
                        }
                        for (index, x) in values.iter().enumerate() {
                            if (values_lock[index] - *x).abs() >= 2 {

                                let app_list = get_app_list().await;

                                //----- MAIN -----
                                if config.apps[index][0] == "_main" {
                                    set_main_volume(x.to_string())
                                }
                                //----- OTHER -----
                                else if config.apps[index][0] == "_other" {
                                    //println!("{:?}", &app_list);
                                    //println!("{:?}", config.apps);

                                    let other_apps: Vec<String> = app_list
                                        .iter()
                                        .filter(|app| {
                                            let mut known_apps = config.apps.iter().flatten();
                                            !known_apps.any(|known| known == &app.name)
                                        })
                                        .map(|app| app.id.clone()) // lub app.to_string() jeśli masz implementację
                                        .collect();

                                    for app in other_apps {
                                        //println!("Found app: {:?}", app);
                                        set_app_volume(app, x.to_string());
                                    }

                                }
                                //----- APP -----
                                else {
                                    for app in app_list {
                                        if config.apps[index].contains(&app.name) {
                                            //println!("Found app: {:?}", app);
                                            set_app_volume(app.id, x.to_string());
                                        }
                                    }
                                }

                                if values_lock.len() > index {
                                    values_lock[index] = values[index].clone();
                                }
                            }
                        }
                    }
                    //Remove used values from buffer
                    read_buffer = read_buffer[pos + 1..].to_string();
                    //Wait till next iteration
                    sleep(Duration::from_millis(config.read_delay)).await;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
            Err(e) => return Err(e.to_string()),
        }
    }
}

//------------------- MAIN ------------------

#[tokio::main]
async fn main() {
    //Get config
    let content = fs::read_to_string("config.json").expect("Couldn't find the config file");
    let mut config: Config = serde_json::from_str(&content).expect("JSON is incorrect");

    //Check if reverse apps order
    if config.reverse {
        config.apps.reverse();
    }

    let show_sessions = get_app_list().await;
    println!("Now avaiable sessions: {:?}.", show_sessions);

    //set_app_volume(1218.to_string(), 50.to_string());

    //println!("Config: {:?}", config);

    // Show currently available apps:
    for (i, app_list) in config.apps.iter().enumerate() {
        println!("App{}: {:?}", i, app_list);
    }

    loop{
        match read_values_from_serial(&config.port, (&config.baud_rate).parse().unwrap(), &config).await {
            Ok(_) => (),
            Err(e) => eprintln!("Błąd: {}", e),
        }

        sleep(Duration::from_secs(5)).await;
    }
}

//------------------- APP LIST ------------------

async fn get_app_list() -> Vec<App> {
    let output = Command::new("pactl")
        .arg("list")
        .arg("sink-inputs")
        .output()
        .expect("Nie udało się wykonać polecenia pactl");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut app_list: Vec<App> = Vec::new();
    let mut iterated_app: App = App {
        id: String::new(),
        name: String::new(),
    };

    let mut last_was_sink_input = false;
    let mut found_binary = false;

    for line in stdout.lines() {
        let line = line.trim();

        if line.starts_with("Sink Input #") {
            if last_was_sink_input && !found_binary {
                let mut other_app = iterated_app.clone();
                other_app.name = "other_app_name_not_avaiable".to_string();
                app_list.push(other_app);
            }

            iterated_app = App {
                id: line.replace("Sink Input #", "").replace("\"", "").trim().to_lowercase(),
                name: String::new(),
            };

            last_was_sink_input = true;
            found_binary = false;
        }

        if line.starts_with("application.process.binary") {
            iterated_app.name = line.replace("application.process.binary =", "").replace("\"", "").trim().to_lowercase();
            app_list.push(iterated_app.clone());
            last_was_sink_input = false;
            found_binary = true;
        }
    }

    // Jeśli ostatni input też był bez .binary, dodajemy na końcu
    if last_was_sink_input && !found_binary {
        let mut other_app = iterated_app.clone();
        other_app.name = "other".to_string();
        app_list.push(other_app);
    }

    app_list
}


//------------------- SET APP VOLUME ------------------

fn set_app_volume(id: String, volume: String) {
    // Preparing command to set app volume
    let status = Command::new("pactl")
        .arg("set-sink-input-volume")
        .arg(&id)
        .arg(format!("{}%", &volume))
        .status()
        .expect("Nie udało się ustawić głośności");
    //Executing app volume
    if status.success() {
        println!("✅ Changed app volume with ID: '{}' to {}%.", id, volume);
    } else {
        eprintln!("❌ Couldn't change app volume with ID: '{}'.", id);
    }
}

//------------------- SET MAIN ------------------
fn set_main_volume (volume: String) {
    Command::new("pactl")
        .arg("set-sink-volume")
        .arg("@DEFAULT_SINK@")
        .arg(format!("{}%", volume))
        .output()
        .expect("Failed to execute command");
}
