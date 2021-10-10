use adsb_deku::deku::DekuContainerRead;
use adsb_deku::{Frame, DF, ME};

use std::io::{self, BufRead, BufReader};
use std::net::TcpStream;
use std::num::ParseFloatError;
use std::str::FromStr;

use apps::Airplanes;

use clap::{AppSettings, Clap};

use tui::backend::{Backend, CrosstermBackend};
use tui::layout::{Constraint, Direction, Layout};
use tui::style::Color;
use tui::widgets::canvas::{Canvas, Line, Points};
use tui::widgets::{Block, Borders};
use tui::Terminal;

pub struct City {
    name: String,
    lat: f64,
    long: f64,
}

impl FromStr for City {
    type Err = ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords: Vec<&str> = s
            .trim_matches(|p| p == '(' || p == ')')
            .split(',')
            .collect();

        let lat_fromstr = coords[1].parse::<f64>()?;
        let long_fromstr = coords[2].parse::<f64>()?;

        Ok(Self {
            name: coords[0].to_string(),
            lat: lat_fromstr,
            long: long_fromstr,
        })
    }
}

#[derive(Clap)]
#[clap(version = "1.0", author = "wcampbell <wcampbell1995@gmail.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    /// Antenna location latitude
    #[clap(long)]
    lat: f64,
    /// Antenna location longitude
    #[clap(long)]
    long: f64,
    /// Vector of cities [(name, lat, long),..]
    #[clap(long)]
    cities: Vec<City>,
    /// Disable output of latitude and longitude on display
    #[clap(long)]
    disable_lat_long: bool,
}

fn main() {
    let opts = Opts::parse();
    let local_lat = opts.lat;
    let local_long = opts.long;
    let cities = opts.cities;
    let disable_lat_long = opts.disable_lat_long;

    let stream = TcpStream::connect(("127.0.0.1", 30002)).unwrap();
    let mut reader = BufReader::new(stream);
    let mut input = String::new();
    let mut airplains = Airplanes::new();

    let stdout = io::stdout();
    let mut backend = CrosstermBackend::new(stdout);
    backend.clear().unwrap();
    let mut terminal = Terminal::new(backend).unwrap();

    loop {
        let len = reader.read_line(&mut input).unwrap();
        let hex = &input.to_string()[1..len - 2];
        let bytes = hex::decode(&hex).unwrap();
        match Frame::from_bytes((&bytes, 0)) {
            Ok((_, frame)) => {
                if let DF::ADSB(ref adsb) = frame.df {
                    if let ME::AirbornePositionBaroAltitude(_) = adsb.me {
                        airplains.add_extended_quitter_ap(adsb.icao, frame.clone());
                    }
                }
            }
            Err(_e) => (),
        }
        input.clear();
        airplains.prune();

        terminal
            .draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([Constraint::Percentage(100)].as_ref())
                    .split(f.size());

                let canvas = Canvas::default()
                    .block(Block::default().title("ADSB").borders(Borders::ALL))
                    .x_bounds([-180.0, 180.0])
                    .y_bounds([-180.0, 180.0])
                    .paint(|ctx| {
                        ctx.layer();

                        // draw cities
                        for city in &cities {
                            let lat = (city.lat - local_lat) * 200.0;
                            let long = (city.long - local_long) * 200.0;

                            // draw city coor
                            ctx.draw(&Points {
                                coords: &[(long, lat)],
                                color: Color::Green,
                            });

                            // draw city name
                            ctx.print(
                                long + 3.0,
                                lat,
                                Box::leak(city.name.to_string().into_boxed_str()),
                                Color::Green,
                            );
                        }

                        // draw airplanes
                        for key in airplains.0.keys() {
                            let value = airplains.lat_long_altitude(*key);
                            if let Some((position, _altitude)) = value {
                                let lat = (position.latitude - local_lat) * 200.0;
                                let long = (position.longitude - local_long) * 200.0;

                                // draw dot on location
                                ctx.draw(&Points {
                                    coords: &[(long, lat)],
                                    color: Color::White,
                                });

                                let name = if !disable_lat_long {
                                    format!(
                                        "{} ({}, {})",
                                        key, position.latitude, position.longitude
                                    )
                                    .into_boxed_str()
                                } else {
                                    format!("{}", key).into_boxed_str()
                                };

                                // draw plane ICAO name
                                ctx.print(long + 3.0, lat, Box::leak(name), Color::White);
                            }
                        }

                        // Draw vertical and horizontal lines
                        ctx.draw(&Line {
                            x1: 180.0,
                            y1: 0.0,
                            x2: -180.0,
                            y2: 0.0,
                            color: Color::White,
                        });
                        ctx.draw(&Line {
                            x1: 0.0,
                            y1: 180.0,
                            x2: 0.0,
                            y2: -180.0,
                            color: Color::White,
                        });
                    });

                f.render_widget(canvas, chunks[0]);
            })
            .unwrap();
    }
}
