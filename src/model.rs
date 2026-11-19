//! Records kept by the payphone manager, sample data and saving/loading.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Kinds of configuration file a phone receives, with the column titles of their rows
/// and the download command that sends them.
pub const FILE_KINDS: &[(&str, &str, &str, &str)] = &[
    ("Rates", "NPA-NXX / Band", "Initial / Additional", "Download Rates"),
    ("Speed Dial", "Key", "Number", "Download Speed Dial"),
    (
        "Priority Parsing",
        "Dial Pattern",
        "Action / Charge",
        "Download Priority Parsing",
    ),
    ("Messages", "Message #", "Text", "Download Messages"),
    ("Rep/Cards", "Key / Card Range", "Number / Accept", "Download Rep/Cards"),
    ("Registers & Options", "Register", "Value", "Download Program"),
    ("Voice", "Prompt", "Recording", "Download Voice"),
];

pub fn file_kind_for_command(cmd: &str) -> Option<&'static str> {
    FILE_KINDS.iter().find(|k| k.3 == cmd).map(|k| k.0)
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigFile {
    pub kind: String,
    pub name: String,
    pub description: String,
    pub rows: Vec<(String, String)>,
    /// For rate files: the real .R94 file sent to the phone over the modem.
    pub binary_path: String,
}

pub const MODELS: &[&str] = &[
    "Series 5",
    "Series 5501",
    "KX-5502",
    "PCM-4",
    "PCM-5",
    "R94-5",
    "R94-4/3",
    "Other",
];

pub const COMMANDS: &[&str] = &[
    "Get Call Counts",
    "Get Alarms",
    "Get Cashbox Totals",
    "Retrieve Call Records (SMDR)",
    "Get Software Versions",
    "Set Date/Time",
    "Set Totalizer",
    "Download Rates",
    "Download Speed Dial",
    "Download Priority Parsing",
    "Download Program",
    "Download Voice",
    "Download Messages",
    "Download Rep/Cards",
    "Get RAM Image",
    "Clear Alarms",
    "Store Settings in EEPROM",
];

pub const ALARM_TYPES: &[&str] = &[
    "Coin box full",
    "Handset tampered",
    "Coin jam",
    "Line voltage low",
    "Keypad failure",
    "Door opened",
    "Memory checksum error",
    "Card reader fault",
];

pub const TIME_ZONES: &[&str] = &["Eastern", "Central", "Mountain", "Pacific", "Alaska", "Hawaii"];

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Customer {
    pub id: u32,
    pub company: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub contact: String,
    pub phone: String,
    pub fax: String,
    pub tax_id: String,
    pub notes: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Contract {
    pub id: u32,
    pub customer_id: Option<u32>,
    pub description: String,
    pub start: String,
    pub end: String,
    pub notify_days: u32,
    pub site_ids: Vec<u32>,
    pub commission_pct: f64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Site {
    pub id: u32,
    pub name: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub contact: String,
    pub phone: String,
    pub map_coord: String,
    pub time_zone: String,
    pub route_id: Option<u32>,
    pub dial_prefix: String,
    pub dial_pattern: String,
    pub user_var: [String; 3],
    pub notes: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Phone {
    pub ani: String,
    pub phone_id: String,
    pub model: String,
    pub site_id: Option<u32>,
    pub location: String,
    pub description: String,
    pub serial: String,
    pub rom_ver: String,
    pub pgm_ver: String,
    pub install_date: String,
    pub password: String,
    pub bypass: String,
    pub phone_type: String,
    pub coll_interval: u32,
    pub cashbox_serial: String,
    pub cashbox_seal: String,
    pub upper_key: String,
    pub lower_key: String,
    pub group: String,
    pub user_def: [String; 3],
    pub notes: String,
    pub tax_template: String,
    pub rate_template: String,
    pub totalizer: f64,
    pub cashbox: f64,
    pub last_collected: f64,
    pub last_contact: String,
    /// Values last read back from the phone (register 400/402/403/404).
    pub ram_bypass: String,
    pub ram_phone_id: String,
    pub ram_password: String,
    pub ram_ani: String,
    pub needs_update: bool,
    /// Configuration file assigned per kind (Rates, Speed Dial...).
    pub files: BTreeMap<String, String>,
    /// When each kind of file was last downloaded to the phone.
    pub file_dates: BTreeMap<String, String>,
    /// What the modem dials for this phone (a number or a sip: address); blank = site prefix + ANI.
    pub dial_target: String,
    /// The phone data block last read over the modem (hex) and when.
    pub ram_block: String,
    pub ram_block_time: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Route {
    pub id: u32,
    pub description: String,
    pub site_ids: Vec<u32>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Technician {
    pub id: u32,
    pub name: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub cb_handle: String,
    pub pin: String,
    pub bus_phone: String,
    pub home_phone: String,
    pub fax: String,
    pub cellular: String,
    pub beeper: String,
    pub route_ids: Vec<u32>,
    pub notes: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WoAction {
    pub date: String,
    pub text: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkOrder {
    pub number: u32,
    pub ani: String,
    pub opened: String,
    pub closed: String,
    pub status: String,
    pub problem: String,
    pub tech_id: Option<u32>,
    pub account_code: String,
    pub amount: f64,
    pub actions: Vec<WoAction>,
    pub notes: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Alarm {
    pub id: u32,
    pub ani: String,
    pub time: String,
    pub description: String,
    pub handled: bool,
    pub handled_note: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PollList {
    pub id: u32,
    pub name: String,
    pub kind: String,
    pub commands: Vec<String>,
    pub phones: Vec<String>,
    pub schedule: Vec<String>,
    pub last_run: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct JournalEntry {
    pub id: u32,
    pub date: String,
    pub ani: String,
    pub account_code: String,
    pub amount: f64,
    pub tech_id: Option<u32>,
    pub notes: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AccountCode {
    pub code: String,
    pub description: String,
    pub kind: String,
    pub amount: f64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Complaint {
    pub id: u32,
    pub date: String,
    pub time: String,
    pub ani: String,
    pub location: String,
    pub name: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub phone: String,
    pub reason: String,
    pub account_code: String,
    pub amount: f64,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CallCounts {
    pub local_coin: u32,
    pub other_coin: u32,
    pub zero_plus: u32,
    pub zero_minus: u32,
    pub dial_around: u32,
    pub debit_card: u32,
    pub dataport: u32,
    pub other: u32,
}

impl CallCounts {
    pub fn total(&self) -> u32 {
        self.local_coin
            + self.other_coin
            + self.zero_plus
            + self.zero_minus
            + self.dial_around
            + self.debit_card
            + self.dataport
            + self.other
    }
    pub fn rows(&self) -> [(&'static str, u32); 8] {
        [
            ("Local Coin", self.local_coin),
            ("Other Coin", self.other_coin),
            ("Zero Plus", self.zero_plus),
            ("Zero Minus", self.zero_minus),
            ("Dial Around", self.dial_around),
            ("Debit Card", self.debit_card),
            ("Dataport Calls", self.dataport),
            ("Other Calls", self.other),
        ]
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Contact {
    pub id: u32,
    pub ani: String,
    pub time: String,
    pub result: String,
    pub commands: Vec<String>,
    pub counts: CallCounts,
    pub totalizer: f64,
    pub cashbox: f64,
    pub alarms: Vec<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CallRecord {
    pub ani: String,
    pub date: String,
    pub call_type: String,
    pub dialed: String,
    pub duration_secs: u32,
    pub coin: f64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TaxTemplate {
    pub name: String,
    pub description: String,
    pub federal: f64,
    pub state: f64,
    pub city: f64,
    pub local: f64,
    pub other: f64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RateTemplate {
    pub name: String,
    pub description: String,
    pub lec_flat: f64,
    pub lec_per_min: f64,
    pub ixc_flat: f64,
    pub ixc_per_min: f64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Macro {
    pub name: String,
    pub description: String,
    pub commands: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub company: String,
    pub user_name: String,
    pub user_id: String,
    pub comm_port: String,
    pub modem_type: String,
    pub init_series5: String,
    pub init_kx: String,
    pub dial_prefix: String,
    pub timeout_secs: u32,
    pub ten_digit_id: bool,
    pub default_password: String,
    pub default_bypass: String,
    pub connection: crate::link::Connection,
    pub baud: u32,
    pub tcp_addr: String,
    pub pnm_codes: crate::link::PnmCodes,
    pub sip_server: String,
    pub sip_port: u16,
    pub sip_user: String,
    pub sip_password: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            company: "Sample Payphone Co.".into(),
            user_name: "Operator".into(),
            user_id: "OP1".into(),
            comm_port: "COM2".into(),
            modem_type: "Hayes compatible 2400".into(),
            init_series5: "AT&FE0V1X4S0=0".into(),
            init_kx: "AT&FE0V1X4S7=60".into(),
            dial_prefix: "ATDT".into(),
            timeout_secs: 45,
            ten_digit_id: false,
            default_password: "1234".into(),
            default_bypass: "0000".into(),
            connection: crate::link::Connection::Serial,
            baud: 9600,
            tcp_addr: "127.0.0.1:2323".into(),
            pnm_codes: crate::link::PnmCodes::default(),
            sip_server: String::new(),
            sip_port: 5060,
            sip_user: String::new(),
            sip_password: String::new(),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Db {
    pub next_id: u32,
    pub settings: Settings,
    pub customers: Vec<Customer>,
    pub contracts: Vec<Contract>,
    pub sites: Vec<Site>,
    pub phones: Vec<Phone>,
    pub routes: Vec<Route>,
    pub techs: Vec<Technician>,
    pub work_orders: Vec<WorkOrder>,
    pub alarms: Vec<Alarm>,
    pub poll_lists: Vec<PollList>,
    pub journal: Vec<JournalEntry>,
    pub account_codes: Vec<AccountCode>,
    pub complaints: Vec<Complaint>,
    pub contacts: Vec<Contact>,
    pub call_records: Vec<CallRecord>,
    pub tax_templates: Vec<TaxTemplate>,
    pub rate_templates: Vec<RateTemplate>,
    pub macros: Vec<Macro>,
    pub config_files: Vec<ConfigFile>,
    pub messages: Vec<String>,
}

pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}
pub fn now_stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
}

pub fn money(v: f64) -> String {
    format!("${:.2}", v)
}

/// Small deterministic random generator for the simulated payphone replies.
pub struct Rng(u64);
impl Rng {
    pub fn seeded(text: &str, salt: u64) -> Self {
        let mut h: u64 = 0xcbf29ce484222325 ^ salt.wrapping_mul(0x9E3779B97F4A7C15);
        for b in text.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Rng(h | 1)
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + (self.next() % ((hi - lo + 1) as u64)) as u32
    }
}

impl Db {
    pub fn new_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    pub fn path() -> PathBuf {
        let base = std::env::var_os("APPDATA")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("PayphoneManager").join("database.json")
    }

    /// The database location for display, without the user's own folder name.
    pub fn path_display() -> String {
        if std::env::var_os("APPDATA").is_some() {
            "%APPDATA%\\PayphoneManager\\database.json".into()
        } else {
            "~/PayphoneManager/database.json".into()
        }
    }

    pub fn load_or_sample() -> Db {
        let p = Db::path();
        if let Ok(text) = std::fs::read_to_string(&p) {
            if let Ok(db) = serde_json::from_str::<Db>(&text) {
                return db;
            }
        }
        Db::sample()
    }

    pub fn save(&self) -> Result<(), String> {
        let p = Db::path();
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&p, text).map_err(|e| e.to_string())
    }

    pub fn export_to(&self, path: &std::path::Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| e.to_string())
    }

    // ---- lookups ----
    pub fn customer(&self, id: Option<u32>) -> Option<&Customer> {
        id.and_then(|id| self.customers.iter().find(|c| c.id == id))
    }
    pub fn contract(&self, id: Option<u32>) -> Option<&Contract> {
        id.and_then(|id| self.contracts.iter().find(|c| c.id == id))
    }
    pub fn site(&self, id: Option<u32>) -> Option<&Site> {
        id.and_then(|id| self.sites.iter().find(|c| c.id == id))
    }
    pub fn route(&self, id: Option<u32>) -> Option<&Route> {
        id.and_then(|id| self.routes.iter().find(|c| c.id == id))
    }
    pub fn tech(&self, id: Option<u32>) -> Option<&Technician> {
        id.and_then(|id| self.techs.iter().find(|c| c.id == id))
    }
    pub fn phone(&self, ani: &str) -> Option<&Phone> {
        self.phones.iter().find(|p| p.ani == ani)
    }
    pub fn phone_mut(&mut self, ani: &str) -> Option<&mut Phone> {
        self.phones.iter_mut().find(|p| p.ani == ani)
    }
    pub fn site_name(&self, id: Option<u32>) -> String {
        self.site(id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "UNASSIGNED".into())
    }
    pub fn tech_name(&self, id: Option<u32>) -> String {
        self.tech(id).map(|s| s.name.clone()).unwrap_or_default()
    }
    pub fn contract_of_site(&self, site_id: u32) -> Option<&Contract> {
        self.contracts.iter().find(|c| c.site_ids.contains(&site_id))
    }
    pub fn tech_of_route(&self, route_id: Option<u32>) -> Option<&Technician> {
        let r = route_id?;
        self.techs.iter().find(|t| t.route_ids.contains(&r))
    }
    pub fn phone_label(&self, ani: &str) -> String {
        match self.phone(ani) {
            Some(p) => format!("{}  {}", p.ani, p.location),
            None => ani.to_string(),
        }
    }
    pub fn open_alarms(&self, ani: &str) -> usize {
        self.alarms.iter().filter(|a| a.ani == ani && !a.handled).count()
    }

    pub fn account_kind(&self, code: &str) -> String {
        self.account_codes
            .iter()
            .find(|a| a.code == code)
            .map(|a| a.kind.clone())
            .unwrap_or_else(|| "Expense".into())
    }

    /// A made-up contact, used only to give the sample database some history.
    pub fn simulate_contact(&mut self, ani: &str, commands: &[String], salt: u64) -> Contact {
        let id = self.new_id();
        let mut rng = Rng::seeded(ani, salt ^ id as u64);
        let mut c = Contact {
            id,
            ani: ani.to_string(),
            time: now_stamp(),
            result: "Completed".into(),
            commands: commands.to_vec(),
            ..Default::default()
        };
        // one in twelve calls gets no answer
        if rng.range(1, 12) == 1 {
            c.result = "NO ANSWER".into();
            self.contacts.push(c.clone());
            return c;
        }
        let wants = |name: &str| commands.iter().any(|x| x.starts_with(name));
        let counts = CallCounts {
            local_coin: rng.range(20, 160),
            other_coin: rng.range(0, 25),
            zero_plus: rng.range(0, 30),
            zero_minus: rng.range(0, 10),
            dial_around: rng.range(0, 40),
            debit_card: rng.range(0, 12),
            dataport: rng.range(0, 6),
            other: rng.range(0, 15),
        };
        let added = counts.local_coin as f64 * 0.35 + counts.other_coin as f64 * 1.25;
        let alarms: Vec<String> = if rng.range(1, 4) == 1 {
            vec![ALARM_TYPES[rng.range(0, ALARM_TYPES.len() as u32 - 1) as usize].to_string()]
        } else {
            vec![]
        };
        let dates: Vec<String> = (0..6)
            .map(|i| {
                (chrono::Local::now() - chrono::Duration::hours(i * 7 + rng.range(0, 6) as i64))
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            })
            .collect();
        let mut smdr = Vec::new();
        if wants("Retrieve Call Records") {
            for d in &dates {
                let kind = ["Local Coin", "Zero Plus", "Dial Around", "Other Coin"][rng.range(0, 3) as usize];
                smdr.push(CallRecord {
                    ani: ani.to_string(),
                    date: d.clone(),
                    call_type: kind.into(),
                    dialed: format!("{}-{:04}", rng.range(200, 989), rng.range(0, 9999)),
                    duration_secs: rng.range(15, 900),
                    coin: if kind.contains("Coin") {
                        0.35 * rng.range(1, 4) as f64
                    } else {
                        0.0
                    },
                });
            }
        }
        let clear_alarms = wants("Clear Alarms");
        let mut downloaded = false;
        if let Some(p) = self.phone_mut(ani) {
            if wants("Get Call Counts") || wants("Get Cashbox") {
                p.cashbox += added;
                p.totalizer += added;
            }
            if wants("Get RAM Image") || wants("Get Software") {
                p.ram_ani = p.ani.clone();
                p.ram_phone_id = p.phone_id.clone();
                p.ram_password = p.password.clone();
                p.ram_bypass = p.bypass.clone();
            }
            if commands.iter().any(|x| x.starts_with("Download")) {
                p.needs_update = false;
                downloaded = true;
                for cmd in commands {
                    if let Some(kind) = file_kind_for_command(cmd) {
                        p.file_dates.insert(kind.to_string(), today());
                    }
                }
            }
            p.last_contact = c.time.clone();
            c.totalizer = p.totalizer;
            c.cashbox = p.cashbox;
        }
        let _ = downloaded;
        if wants("Get Call Counts") {
            c.counts = counts;
        }
        if wants("Get Alarms") {
            c.alarms = alarms.clone();
            for a in alarms {
                let aid = self.new_id();
                self.alarms.push(Alarm {
                    id: aid,
                    ani: ani.to_string(),
                    time: c.time.clone(),
                    description: a,
                    handled: false,
                    handled_note: String::new(),
                });
            }
        }
        if clear_alarms {
            for a in self.alarms.iter_mut().filter(|a| a.ani == ani && !a.handled) {
                a.handled = true;
                a.handled_note = "Cleared at the phone".into();
            }
        }
        self.call_records.extend(smdr);
        self.contacts.push(c.clone());
        c
    }

    pub fn sample() -> Db {
        let mut db = Db {
            settings: Settings::default(),
            ..Default::default()
        };
        let customers = [
            (
                "Riverside Markets Inc.",
                "410 Elm Street",
                "Springfield",
                "IL",
                "62701",
                "Dana Whitfield",
                "217-555-0142",
            ),
            (
                "Northgate Transit Authority",
                "88 Depot Road",
                "Peoria",
                "IL",
                "61602",
                "Sam Okafor",
                "309-555-0177",
            ),
            (
                "Lakeview Hospitality Group",
                "12 Shore Drive",
                "Bloomington",
                "IL",
                "61701",
                "Pat Lindqvist",
                "309-555-0110",
            ),
        ];
        for (co, ad, ci, st, zp, ct, ph) in customers {
            let id = db.new_id();
            db.customers.push(Customer {
                id,
                company: co.into(),
                address: ad.into(),
                city: ci.into(),
                state: st.into(),
                zip: zp.into(),
                contact: ct.into(),
                phone: ph.into(),
                fax: String::new(),
                tax_id: format!("36-{:07}", id * 104729 % 9999999),
                notes: String::new(),
            });
        }
        let techs = [
            ("Chris Navarro", "Springfield", "217-555-0190", "217-555-0300"),
            ("Morgan Ellis", "Peoria", "309-555-0123", "309-555-0311"),
        ];
        for (n, ci, b, bp) in techs {
            let id = db.new_id();
            db.techs.push(Technician {
                id,
                name: n.into(),
                address: "PO Box 1".into(),
                city: ci.into(),
                state: "IL".into(),
                zip: "62700".into(),
                cb_handle: String::new(),
                pin: format!("{}", 4000 + id),
                bus_phone: b.into(),
                home_phone: String::new(),
                fax: String::new(),
                cellular: String::new(),
                beeper: bp.into(),
                route_ids: vec![],
                notes: String::new(),
            });
        }
        let routes = ["North Loop", "Downtown", "Lakeside"];
        for r in routes {
            let id = db.new_id();
            db.routes.push(Route {
                id,
                description: r.into(),
                site_ids: vec![],
            });
        }
        db.techs[0].route_ids = vec![db.routes[0].id, db.routes[1].id];
        db.techs[1].route_ids = vec![db.routes[2].id];

        let sites = [
            ("Riverside Market #1", "410 Elm Street", "Springfield", 0usize, 0usize),
            ("Riverside Market #2", "1201 Monroe Ave", "Springfield", 0, 1),
            ("Depot Station", "88 Depot Road", "Peoria", 1, 1),
            ("Main Street Bus Shelter", "300 Main Street", "Peoria", 1, 0),
            ("Lakeview Inn Lobby", "12 Shore Drive", "Bloomington", 2, 2),
            ("Marina Snack Bar", "2 Harbor Way", "Bloomington", 2, 2),
        ];
        let mut site_ids = vec![];
        for (n, ad, ci, cust, route) in sites {
            let id = db.new_id();
            site_ids.push((id, cust));
            let rid = db.routes[route].id;
            db.routes[route].site_ids.push(id);
            db.sites.push(Site {
                id,
                name: n.into(),
                address: ad.into(),
                city: ci.into(),
                state: "IL".into(),
                zip: "62701".into(),
                contact: "Store manager".into(),
                phone: format!("217-555-{:04}", 1000 + id),
                map_coord: format!("{}-{}", (b'A' + (id % 8) as u8) as char, id % 20),
                time_zone: "Central".into(),
                route_id: Some(rid),
                dial_prefix: String::new(),
                dial_pattern: "1+NPA+NXX+XXXX".into(),
                user_var: Default::default(),
                notes: String::new(),
            });
        }
        for (i, c) in db.customers.clone().iter().enumerate() {
            let id = db.new_id();
            db.contracts.push(Contract {
                id,
                customer_id: Some(c.id),
                description: format!(
                    "{} location agreement",
                    c.company.split_whitespace().next().unwrap_or("")
                ),
                start: format!("2024-0{}-01", i + 1),
                end: format!("2027-0{}-01", i + 1),
                notify_days: 60,
                site_ids: site_ids.iter().filter(|(_, cu)| *cu == i).map(|(s, _)| *s).collect(),
                commission_pct: 15.0 + 5.0 * i as f64,
            });
        }
        let phones = [
            ("217-555-2010", "Front entrance", "Series 5", 0usize),
            ("217-555-2011", "Rear by carts", "KX-5502", 0),
            ("217-555-2044", "Checkout lanes", "Series 5", 1),
            ("309-555-3100", "Platform A", "KX-5502", 2),
            ("309-555-3101", "Platform B", "KX-5502", 2),
            ("309-555-3150", "Shelter north side", "Series 5501", 3),
            ("309-555-4410", "Lobby by elevators", "PCM-5", 4),
            ("309-555-4477", "Dock entrance", "R94-5", 5),
        ];
        for (k, (ani, loc, model, site)) in phones.iter().enumerate() {
            let sid = site_ids[*site].0;
            db.phones.push(Phone {
                ani: ani.to_string(),
                phone_id: format!("{:06}", 100200 + k * 17),
                model: model.to_string(),
                site_id: Some(sid),
                location: loc.to_string(),
                description: format!("{} single-line coin phone", model),
                serial: format!("SN{:07}", 5550000 + k * 131),
                rom_ver: "5.3.3".into(),
                pgm_ver: if model.starts_with("KX") {
                    "KX 1.3.0".into()
                } else {
                    "5.3.1".into()
                },
                install_date: format!("2023-{:02}-15", k % 12 + 1),
                password: "1234".into(),
                bypass: "0000".into(),
                phone_type: "Coin".into(),
                coll_interval: 14,
                cashbox_serial: format!("CB{:05}", 300 + k),
                cashbox_seal: format!("{:06}", 811000 + k * 7),
                upper_key: "A1".into(),
                lower_key: "B2".into(),
                group: "1".into(),
                user_def: Default::default(),
                notes: String::new(),
                tax_template: "Illinois Standard".into(),
                rate_template: "Standard LEC/LD".into(),
                totalizer: 1200.0 + k as f64 * 310.25,
                cashbox: 40.0 + k as f64 * 12.5,
                last_collected: 85.0 + k as f64 * 3.0,
                last_contact: String::new(),
                ram_bypass: "0000".into(),
                ram_phone_id: format!("{:06}", 100200 + k * 17),
                ram_password: if k == 3 { "9999".into() } else { "1234".into() },
                ram_ani: ani.to_string(),
                needs_update: k == 6,
                ..Default::default()
            });
        }
        db.account_codes = vec![
            AccountCode {
                code: "100".into(),
                description: "Coin collection".into(),
                kind: "Income".into(),
                amount: 0.0,
            },
            AccountCode {
                code: "110".into(),
                description: "Commission income (LD)".into(),
                kind: "Income".into(),
                amount: 0.0,
            },
            AccountCode {
                code: "200".into(),
                description: "Repair parts".into(),
                kind: "Expense".into(),
                amount: 25.0,
            },
            AccountCode {
                code: "210".into(),
                description: "Service call labor".into(),
                kind: "Expense".into(),
                amount: 45.0,
            },
            AccountCode {
                code: "220".into(),
                description: "Customer refund".into(),
                kind: "Expense".into(),
                amount: 0.35,
            },
            AccountCode {
                code: "300".into(),
                description: "Line charges".into(),
                kind: "Expense".into(),
                amount: 38.0,
            },
        ];
        db.tax_templates = vec![
            TaxTemplate {
                name: "Illinois Standard".into(),
                description: "State, city and federal".into(),
                federal: 3.0,
                state: 7.0,
                city: 2.5,
                local: 0.0,
                other: 0.0,
            },
            TaxTemplate {
                name: "No Tax".into(),
                description: "Exempt locations".into(),
                ..Default::default()
            },
        ];
        db.rate_templates = vec![
            RateTemplate {
                name: "Standard LEC/LD".into(),
                description: "Default charges".into(),
                lec_flat: 0.50,
                lec_per_min: 0.0,
                ixc_flat: 0.0,
                ixc_per_min: 0.25,
            },
            RateTemplate {
                name: "Transit Discount".into(),
                description: "Bus and rail stations".into(),
                lec_flat: 0.35,
                lec_per_min: 0.0,
                ixc_flat: 0.0,
                ixc_per_min: 0.20,
            },
        ];
        db.macros = vec![
            Macro {
                name: "Daily Check".into(),
                description: "Counts, alarms and cashbox".into(),
                commands: vec![
                    "Get Call Counts".into(),
                    "Get Alarms".into(),
                    "Get Cashbox Totals".into(),
                ],
            },
            Macro {
                name: "Full Update".into(),
                description: "Send all operational files".into(),
                commands: vec![
                    "Download Rates".into(),
                    "Download Speed Dial".into(),
                    "Download Priority Parsing".into(),
                    "Download Messages".into(),
                ],
            },
        ];
        let r = |a: &str, b: &str| (a.to_string(), b.to_string());
        db.config_files = vec![
            ConfigFile {
                kind: "Rates".into(),
                name: "IL217".into(),
                description: "Central Illinois rates".into(),
                rows: vec![
                    r("217-555 local", "$0.50 / $0.10"),
                    r("309 intraLATA", "$0.90 / $0.25"),
                    r("Band 3 interstate", "$1.75 / $0.40"),
                ],
                ..Default::default()
            },
            ConfigFile {
                kind: "Rates".into(),
                name: "IL309".into(),
                description: "Peoria area rates".into(),
                rows: vec![r("309-555 local", "$0.50 / $0.10"), r("217 intraLATA", "$0.90 / $0.25")],
                ..Default::default()
            },
            ConfigFile {
                kind: "Speed Dial".into(),
                name: "STANDARD".into(),
                description: "Repair and taxi".into(),
                rows: vec![r("1", "800-555-0100 Repair"), r("2", "217-555-0123 Taxi")],
                ..Default::default()
            },
            ConfigFile {
                kind: "Priority Parsing".into(),
                name: "DEFAULT".into(),
                description: "Free and blocked numbers".into(),
                rows: vec![
                    r("911", "Free call"),
                    r("1-900-+", "Blocked"),
                    r("1-800-+", "Free call"),
                ],
                ..Default::default()
            },
            ConfigFile {
                kind: "Messages".into(),
                name: "WELCOME".into(),
                description: "Scrolling messages".into(),
                rows: vec![r("1", "LOCAL CALLS 50 CENTS"), r("2", "HAVE A NICE DAY")],
                ..Default::default()
            },
            ConfigFile {
                kind: "Rep/Cards".into(),
                name: "KXCARDS".into(),
                description: "KX-5502 rep dialer and cards".into(),
                rows: vec![r("Rep 1", "800-555-0199"), r("Cards 4000-4999", "Accept")],
                ..Default::default()
            },
            ConfigFile {
                kind: "Registers & Options".into(),
                name: "STD-COIN".into(),
                description: "Standard coin phone template".into(),
                rows: vec![
                    r("400 Owner Bypass", "0000"),
                    r("410 Local call rate", "50"),
                    r("442 Good cards to store", "05"),
                ],
                ..Default::default()
            },
            ConfigFile {
                kind: "Voice".into(),
                name: "ENGLISH".into(),
                description: "Standard prompts".into(),
                rows: vec![r("Deposit", "Please deposit 50 cents")],
                ..Default::default()
            },
        ];
        for p in db.phones.iter_mut() {
            let kx = p.model.starts_with("KX");
            let rate = if p.ani.starts_with("217") { "IL217" } else { "IL309" };
            for (kind, name) in [
                ("Rates", rate),
                ("Speed Dial", "STANDARD"),
                ("Priority Parsing", "DEFAULT"),
                ("Messages", "WELCOME"),
                ("Registers & Options", "STD-COIN"),
                ("Voice", "ENGLISH"),
            ] {
                p.files.insert(kind.into(), name.into());
                p.file_dates.insert(kind.into(), p.install_date.clone());
            }
            if kx {
                p.files.insert("Rep/Cards".into(), "KXCARDS".into());
                p.file_dates.insert("Rep/Cards".into(), p.install_date.clone());
            }
        }
        let anis: Vec<String> = db.phones.iter().map(|p| p.ani.clone()).collect();
        let pl_id = db.new_id();
        db.poll_lists.push(PollList {
            id: pl_id,
            name: "Nightly Counts".into(),
            kind: "Recurring".into(),
            commands: vec![
                "Get Call Counts".into(),
                "Get Alarms".into(),
                "Get Cashbox Totals".into(),
            ],
            phones: anis.clone(),
            schedule: vec!["0200".into()],
            last_run: String::new(),
        });
        let pl2 = db.new_id();
        db.poll_lists.push(PollList {
            id: pl2,
            name: "Rate Update - Lakeside".into(),
            kind: "One-time".into(),
            commands: vec!["Download Rates".into()],
            phones: vec![anis[6].clone(), anis[7].clone()],
            schedule: vec!["2330".into()],
            last_run: String::new(),
        });
        // history: a few contacts so diagnostics have something to show
        for (i, a) in anis.iter().enumerate() {
            let cmds: Vec<String> = vec![
                "Get Call Counts".into(),
                "Get Alarms".into(),
                "Retrieve Call Records (SMDR)".into(),
            ];
            let c = db.simulate_contact(a, &cmds, 7 + i as u64);
            let _ = c;
        }
        // guarantee some open alarms for the status screen
        for (ani, what) in [
            (&anis[1], "Coin box full"),
            (&anis[4], "Handset tampered"),
            (&anis[6], "Line voltage low"),
        ] {
            if db.open_alarms(ani) == 0 {
                let id = db.new_id();
                db.alarms.push(Alarm {
                    id,
                    ani: ani.clone(),
                    time: now_stamp(),
                    description: what.into(),
                    handled: false,
                    handled_note: String::new(),
                });
            }
        }
        let t0 = db.techs[0].id;
        let wo = db.new_id();
        db.work_orders.push(WorkOrder {
            number: 1001,
            ani: anis[1].clone(),
            opened: today(),
            closed: String::new(),
            status: "Open".into(),
            problem: "Coin box full - needs collection".into(),
            tech_id: Some(t0),
            account_code: "210".into(),
            amount: 45.0,
            actions: vec![WoAction {
                date: today(),
                text: "Dispatched technician".into(),
            }],
            notes: String::new(),
        });
        let _ = wo;
        db.work_orders.push(WorkOrder {
            number: 1000,
            ani: anis[3].clone(),
            opened: "2026-10-28".into(),
            closed: "2026-10-29".into(),
            status: "Closed".into(),
            problem: "Dead keypad row".into(),
            tech_id: Some(db.techs[1].id),
            account_code: "200".into(),
            amount: 25.0,
            actions: vec![WoAction {
                date: "2026-10-29".into(),
                text: "Replaced keypad".into(),
            }],
            notes: String::new(),
        });
        let entries = [
            ("2026-11-02", 0usize, "100", 142.75),
            ("2026-11-02", 3, "100", 98.40),
            ("2026-11-05", 3, "200", 25.0),
            ("2026-11-09", 6, "100", 61.10),
            ("2026-11-10", 1, "300", 38.0),
            ("2026-11-12", 4, "100", 77.20),
        ];
        for (d, p, code, amt) in entries {
            let id = db.new_id();
            db.journal.push(JournalEntry {
                id,
                date: d.into(),
                ani: anis[p].clone(),
                account_code: code.into(),
                amount: amt,
                tech_id: Some(t0),
                notes: String::new(),
            });
        }
        let cid = db.new_id();
        db.complaints.push(Complaint {
            id: cid,
            date: "2026-11-11".into(),
            time: "14:20".into(),
            ani: anis[2].clone(),
            location: "Checkout lanes".into(),
            name: "J. Rivera".into(),
            address: "55 Oak Lane".into(),
            city: "Springfield".into(),
            state: "IL".into(),
            zip: "62702".into(),
            phone: "217-555-0199".into(),
            reason: "Lost coins, call did not connect".into(),
            account_code: "220".into(),
            amount: 0.70,
        });
        db.messages.push(format!("{}  Database opened", now_stamp()));
        db
    }
}
