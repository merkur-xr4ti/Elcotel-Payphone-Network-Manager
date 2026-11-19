//! Which windows are open and what each one is editing.

use crate::model::*;

#[derive(Default)]
pub struct Windows {
    pub menu: bool,
    pub nav: bool,
    pub business: bool,
    pub service: bool,
    pub config: bool,
    pub find: bool,
    pub dial: bool,
    pub status: bool,
    pub handle: bool,
    pub wo: bool,
    pub complaints: bool,
    pub diag: bool,
    pub conhist: bool,
    pub ram: bool,
    pub journal: bool,
    pub commission: bool,
    pub rate_templates: bool,
    pub tax_templates: bool,
    pub accounts: bool,
    pub poll: bool,
    pub sysconfig: bool,
    pub modem: bool,
    pub macros: bool,
    pub global: bool,
    pub purge: bool,
    pub stats: bool,
    pub about: bool,
    pub info: bool,
    pub backup: bool,
    pub files: bool,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum NavGroup {
    Customer,
    Contract,
    Site,
    #[default]
    Phone,
    Route,
    Tech,
}

#[derive(Default)]
pub struct NavState {
    pub group: NavGroup,
    pub customer: Option<u32>,
    pub contract: Option<u32>,
    pub site: Option<u32>,
    pub phone: Option<String>,
    pub route: Option<u32>,
    pub tech: Option<u32>,
    pub phone_by_location: bool,
    pub filter: String,
}

/// A two-list "Add >> / << Remove" picker.
#[derive(Default, Clone)]
pub struct Picker<K: Clone + PartialEq> {
    pub left_sel: Option<K>,
    pub right_sel: Option<K>,
}

#[derive(Default)]
pub struct Editors {
    pub customer: Option<Customer>,
    pub contract: Option<Contract>,
    pub contract_sites: bool,
    pub contract_pick: Picker<u32>,
    pub site: Option<Site>,
    pub site_phones: Option<Vec<String>>,
    pub site_pick: Picker<String>,
    pub phone: Option<Phone>,
    pub phone_orig: String,
    pub phone_polls: bool,
    pub phone_poll_sel: Option<u32>,
    pub route: Option<Route>,
    pub route_pick: Picker<u32>,
    pub tech: Option<Technician>,
    pub notes_for: Option<(String, String)>,
    pub error: String,
}

#[derive(Default)]
pub struct FindState {
    pub ani: String,
    pub phone_id: String,
    pub site: String,
    pub results: Vec<String>,
    pub sel: Option<String>,
    pub searched: bool,
}

#[derive(Default)]
pub struct DialState {
    pub ani: Option<String>,
    pub selected: Vec<String>,
    pub log: Vec<String>,
    pub result: Option<Contact>,
    /// A real call in progress (modem connection).
    pub job: Option<CallJob>,
    /// Calls waiting their turn (poll lists, Redial All).
    pub queue: std::collections::VecDeque<QueuedCall>,
    pub status: String,
    /// Raw command byte (hex) to send, for trying commands.
    pub raw_cmd: String,
    pub last_report: Option<crate::link::Report>,
}

pub struct QueuedCall {
    pub ani: String,
    pub commands: Vec<String>,
    pub raw: Option<u8>,
    pub source: String,
}

pub struct CallJob {
    pub ani: String,
    pub commands: Vec<String>,
    pub source: String,
    pub rx: std::sync::mpsc::Receiver<crate::link::LinkEvent>,
    pub cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub started: std::time::Instant,
}

/// The modem test in Modem settings.
#[derive(Default)]
pub struct ModemTest {
    pub running: Option<std::sync::mpsc::Receiver<String>>,
    pub lines: Vec<String>,
}

#[derive(Default)]
pub struct ServiceState {
    pub status_sel: Option<u32>,
    pub status_models: Vec<String>,
    pub show_filter: bool,
    pub handle_alarm: Option<u32>,
    pub handle_note: String,
    pub wo_sel: Option<u32>,
    pub wo_from: String,
    pub wo_to: String,
    pub wo_status: String,
    pub wo_edit: Option<WorkOrder>,
    pub wo_new: bool,
    pub wo_action: String,
    pub wo_close_it: bool,
    pub complaint_sel: Option<u32>,
    pub complaint_edit: Option<Complaint>,
    pub letter: Option<String>,
    pub diag_ani: Option<String>,
    pub diag_type: String,
    pub conhist_ani: Option<String>,
    pub conhist_sel: Option<u32>,
    pub ram_ani: Option<String>,
}

#[derive(Default)]
pub struct BusinessState {
    pub j_from: String,
    pub j_to: String,
    pub j_sel: Option<u32>,
    pub j_edit: Option<JournalEntry>,
    pub j_summary: bool,
    pub comm_from: String,
    pub comm_to: String,
    pub comm_pick: Picker<u32>,
    pub comm_chosen: Vec<u32>,
    pub comm_result: Vec<(String, f64, u32, f64, f64)>,
    pub tax_sel: Option<String>,
    pub tax_edit: Option<(TaxTemplate, String)>,
    pub rate_sel: Option<String>,
    pub rate_edit: Option<(RateTemplate, String)>,
    pub acct_sel: Option<String>,
    pub acct_edit: Option<(AccountCode, String)>,
}

#[derive(Default)]
pub struct PollState {
    pub sel: Option<u32>,
    pub edit_name: String,
    pub phones_for: Option<u32>,
    pub phones_pick: Picker<String>,
    pub cmds_for: Option<u32>,
    pub cmd_pick: Picker<String>,
    pub sched_for: Option<u32>,
    pub sched_hour: String,
    pub new_list: Option<PollList>,
}

#[derive(Default)]
pub struct ConfigState {
    pub settings: Settings,
    pub macro_sel: Option<String>,
    pub macro_edit: Option<(Macro, String)>,
    pub macro_pick: Picker<String>,
    pub global_model: String,
    pub global_chosen: Vec<String>,
    pub global_pick: Picker<String>,
    pub global_tax: String,
    pub global_rate: String,
    pub global_poll: Option<u32>,
    pub global_macro: String,
    pub purge_from: String,
    pub purge_to: String,
    pub purge_codes: Vec<String>,
    pub backup_path: String,
    pub files_kind: String,
    pub files_sel: Option<String>,
    pub file_edit: Option<(ConfigFile, String)>,
    pub modem_test: ModemTest,
    /// SIP port as typed in Modem Settings.
    pub sip_port_text: String,
    pub ports: Vec<String>,
    /// Command codes being edited, as hex text (blank = not known).
    pub code_text: Vec<String>,
}
