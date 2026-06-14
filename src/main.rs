use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const FIXTURE_JSON: &str = include_str!("../fixtures/demo.json");
const DEFAULT_TEST_URL: &str = "https://test.api.resly.com.au";
const DEFAULT_PRODUCTION_URL: &str = "https://api.resly.com.au";
const APPROVAL_EXPIRY_SECS: u64 = 15 * 60;

type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
struct AppError {
    code: &'static str,
    message: String,
    retryable: bool,
}

impl AppError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retryable: false,
        }
    }

    fn retryable(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retryable: true,
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::new("io.error", value.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::new("json.error", value.to_string())
    }
}

impl From<toml::de::Error> for AppError {
    fn from(value: toml::de::Error) -> Self {
        Self::new("config.invalid", value.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        if value.is_timeout() || value.is_connect() {
            Self::retryable("network.error", value.to_string())
        } else {
            Self::new("http.error", value.to_string())
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "resly")]
#[command(version)]
#[command(about = "Demo CLI for Resly Open API workflows")]
#[command(
    long_about = "A fixture-backed and live-capable CLI for Resly support, partner onboarding, pricing, reservation, inventory, and webhook workflows."
)]
struct Cli {
    #[arg(long, global = true, help = "Emit stable JSON envelopes")]
    json: bool,

    #[arg(
        long,
        global = true,
        help = "Force local demo fixtures instead of live API calls"
    )]
    fixture: bool,

    #[arg(
        long,
        global = true,
        env = "RESLY_BASE_URL",
        help = "Override the Resly base URL"
    )]
    base_url: Option<String>,

    #[arg(
        long,
        global = true,
        env = "RESLY_ACCOUNT_ID",
        help = "Resly account ID"
    )]
    account_id: Option<String>,

    #[arg(long, global = true, env = "RESLY_API_KEY", help = "Resly API key")]
    api_key: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum Commands {
    #[command(about = "Check config, auth, endpoint selection, and fixture fallback")]
    Doctor,
    #[command(about = "Write ~/.resly/config.toml")]
    Init(InitArgs),
    #[command(about = "Retrieve or inspect access tokens")]
    Token {
        #[command(subcommand)]
        command: TokenCommands,
    },
    #[command(about = "Approve or inspect exact write previews before live apply")]
    Approvals {
        #[command(subcommand)]
        command: ApprovalCommands,
    },
    #[command(about = "Read property/account details")]
    Account {
        #[command(subcommand)]
        command: AccountCommands,
    },
    #[command(about = "Read agents and channels")]
    Agents {
        #[command(subcommand)]
        command: AgentsCommands,
    },
    #[command(about = "Read room types/listings")]
    RoomTypes {
        #[command(subcommand)]
        command: RoomTypeCommands,
    },
    #[command(about = "Read physical rooms")]
    Rooms {
        #[command(subcommand)]
        command: RoomCommands,
    },
    #[command(about = "Read rate plans")]
    RatePlans {
        #[command(subcommand)]
        command: RatePlanCommands,
    },
    #[command(about = "Read reservations")]
    Reservations {
        #[command(subcommand)]
        command: ReservationCommands,
    },
    #[command(about = "Answer manager-friendly availability questions")]
    Availability {
        #[command(subcommand)]
        command: AvailabilityCommands,
    },
    #[command(about = "Read room blocks")]
    Blocks {
        #[command(subcommand)]
        command: BlockCommands,
    },
    #[command(about = "Read inventory by room type")]
    Inventory {
        #[command(subcommand)]
        command: InventoryCommands,
    },
    #[command(about = "Read or preview pricing/rate restrictions")]
    Rates {
        #[command(subcommand)]
        command: RateCommands,
    },
    #[command(about = "Read or dry-run webhook setup")]
    Webhooks {
        #[command(subcommand)]
        command: WebhookCommands,
    },
    #[command(about = "Raw read-only API escape hatch")]
    Request {
        #[command(subcommand)]
        command: RequestCommands,
    },
}

#[derive(Args, Debug)]
struct InitArgs {
    #[arg(long)]
    account_id: String,
    #[arg(long)]
    api_key: String,
    #[arg(long, value_enum, default_value = "test")]
    env: ReslyEnvironment,
    #[arg(long)]
    base_url: Option<String>,
}

#[derive(ValueEnum, Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ReslyEnvironment {
    Test,
    Production,
}

impl ReslyEnvironment {
    fn label(self) -> &'static str {
        match self {
            ReslyEnvironment::Test => "test",
            ReslyEnvironment::Production => "production",
        }
    }

    fn base_url(self) -> &'static str {
        match self {
            ReslyEnvironment::Test => DEFAULT_TEST_URL,
            ReslyEnvironment::Production => DEFAULT_PRODUCTION_URL,
        }
    }
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum TokenCommands {
    #[command(about = "Retrieve and cache a live Bearer token")]
    Refresh,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum ApprovalCommands {
    #[command(about = "List pending, approved, and used local write approvals")]
    List,
    #[command(about = "Show one local write approval request")]
    Show(IdArg),
    #[command(about = "Approve one exact write preview and print a one-time token")]
    Approve(ApprovalApproveArgs),
    #[command(about = "Revoke one local write approval request")]
    Revoke(IdArg),
}

#[derive(Args, Debug)]
struct ApprovalApproveArgs {
    id: String,
    #[arg(
        long,
        help = "Must exactly match '<METHOD> <ENDPOINT>' from the preview"
    )]
    confirm_operation: String,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum AccountCommands {
    Get,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum AgentsCommands {
    List,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum RoomTypeCommands {
    List(RoomTypesListArgs),
    Get(IdArg),
}

#[derive(Args, Debug)]
struct RoomTypesListArgs {
    #[arg(long)]
    show_photos: bool,
    #[arg(long)]
    portfolio_id: Option<String>,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum RoomCommands {
    List,
    Get(IdArg),
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum RatePlanCommands {
    List,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum ReservationCommands {
    List(ReservationListArgs),
    Get(IdArg),
    InHouse,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum AvailabilityCommands {
    #[command(about = "Quote available room types for a guest count and stay dates")]
    Quote(AvailabilityQuoteArgs),
}

#[derive(Args, Debug)]
struct AvailabilityQuoteArgs {
    #[arg(long)]
    guests: u64,
    #[arg(long)]
    from: String,
    #[arg(long)]
    to: String,
    #[arg(long, default_value_t = 5)]
    limit: usize,
}

#[derive(Args, Debug)]
struct ReservationListArgs {
    #[arg(long, value_enum, default_value = "check-in")]
    date_type: ReservationDateType,
    #[arg(long, help = "Start date for check-in/check-out queries, YYYY-MM-DD")]
    from: Option<String>,
    #[arg(long, help = "End date for check-in/check-out queries, YYYY-MM-DD")]
    to: Option<String>,
    #[arg(long, help = "Start timestamp for updated queries")]
    start_time: Option<String>,
    #[arg(long, help = "End timestamp for updated queries")]
    end_time: Option<String>,
    #[arg(long, default_value = "confirmed")]
    status: String,
    #[arg(long)]
    room_id: Option<String>,
    #[arg(long, default_value_t = 50)]
    limit: usize,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum ReservationDateType {
    CheckIn,
    CheckOut,
    Updated,
}

impl ReservationDateType {
    fn api_value(self) -> &'static str {
        match self {
            ReservationDateType::CheckIn => "checkIn",
            ReservationDateType::CheckOut => "checkOut",
            ReservationDateType::Updated => "updated",
        }
    }
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum BlockCommands {
    List(BlockListArgs),
}

#[derive(Args, Debug)]
struct BlockListArgs {
    #[arg(long, value_enum, default_value = "in-between")]
    date_type: BlockDateType,
    #[arg(long)]
    from: String,
    #[arg(long)]
    to: String,
    #[arg(long)]
    room_id: Option<String>,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum BlockDateType {
    Start,
    End,
    InBetween,
}

impl BlockDateType {
    fn api_value(self) -> &'static str {
        match self {
            BlockDateType::Start => "start",
            BlockDateType::End => "end",
            BlockDateType::InBetween => "inBetween",
        }
    }
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum InventoryCommands {
    Get(InventoryArgs),
}

#[derive(Args, Debug)]
struct InventoryArgs {
    #[arg(long)]
    room_type: String,
    #[arg(long)]
    from: String,
    #[arg(long)]
    to: String,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum RateCommands {
    Get(RateReadArgs),
    Preview(RateFileArgs),
    Update(RateUpdateArgs),
}

#[derive(Args, Debug)]
struct RateReadArgs {
    #[arg(long)]
    rate_plan: String,
    #[arg(long)]
    from: String,
    #[arg(long)]
    to: String,
}

#[derive(Args, Debug)]
struct RateFileArgs {
    #[arg(long)]
    rate_plan: String,
    #[arg(long)]
    file: PathBuf,
}

#[derive(Args, Debug)]
struct RateUpdateArgs {
    #[arg(long)]
    rate_plan: String,
    #[arg(long)]
    file: PathBuf,
    #[arg(long, help = "Allow a live PATCH only when using the test API")]
    test_only: bool,
    #[arg(
        long,
        help = "Allow a live PATCH; otherwise this command previews only"
    )]
    live: bool,
    #[arg(long, help = "Required for production live writes")]
    confirm_account: Option<String>,
    #[arg(long, help = "Required as 'production' for production live writes")]
    confirm_environment: Option<String>,
    #[arg(long, help = "Approval ID returned by rates preview/update dry run")]
    approval_id: Option<String>,
    #[arg(long, help = "One-time token returned by resly approvals approve")]
    approval_token: Option<String>,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum WebhookCommands {
    List,
    Get(IdArg),
    Create(WebhookCreateArgs),
    Update(WebhookUpdateArgs),
    Delete(WebhookDeleteArgs),
}

#[derive(Args, Debug)]
struct WebhookCreateArgs {
    #[arg(long = "type", value_enum)]
    hook_type: WebhookType,
    #[arg(long)]
    url: String,
    #[arg(long)]
    basic_auth_username: Option<String>,
    #[arg(long)]
    basic_auth_password: Option<String>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    live: bool,
    #[arg(long, help = "Required for production live writes")]
    confirm_account: Option<String>,
    #[arg(long, help = "Required as 'production' for production live writes")]
    confirm_environment: Option<String>,
    #[arg(long, help = "Approval ID returned by webhook dry run")]
    approval_id: Option<String>,
    #[arg(long, help = "One-time token returned by resly approvals approve")]
    approval_token: Option<String>,
}

#[derive(Args, Debug)]
struct WebhookUpdateArgs {
    id: String,
    #[arg(long)]
    url: Option<String>,
    #[arg(long = "type", value_enum)]
    hook_type: Option<WebhookType>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    live: bool,
    #[arg(long, help = "Required for production live writes")]
    confirm_account: Option<String>,
    #[arg(long, help = "Required as 'production' for production live writes")]
    confirm_environment: Option<String>,
    #[arg(long, help = "Approval ID returned by webhook dry run")]
    approval_id: Option<String>,
    #[arg(long, help = "One-time token returned by resly approvals approve")]
    approval_token: Option<String>,
}

#[derive(Args, Debug)]
struct WebhookDeleteArgs {
    id: String,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    live: bool,
    #[arg(
        long,
        help = "Required for live webhook deletes; must match the webhook ID"
    )]
    confirm_delete: Option<String>,
    #[arg(long, help = "Required for production live writes")]
    confirm_account: Option<String>,
    #[arg(long, help = "Required as 'production' for production live writes")]
    confirm_environment: Option<String>,
    #[arg(long, help = "Approval ID returned by webhook dry run")]
    approval_id: Option<String>,
    #[arg(long, help = "One-time token returned by resly approvals approve")]
    approval_token: Option<String>,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum WebhookType {
    Reservations,
    Blocks,
    Rooms,
    Messages,
    RoomTypes,
}

impl WebhookType {
    fn api_value(self) -> &'static str {
        match self {
            WebhookType::Reservations => "reservations",
            WebhookType::Blocks => "blocks",
            WebhookType::Rooms => "rooms",
            WebhookType::Messages => "messages",
            WebhookType::RoomTypes => "roomTypes",
        }
    }
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
enum RequestCommands {
    Get(RawGetArgs),
}

#[derive(Args, Debug)]
struct RawGetArgs {
    path: String,
}

#[derive(Args, Debug)]
struct IdArg {
    id: String,
}

#[derive(Deserialize, Debug, Default)]
struct FileConfig {
    account_id: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    environment: Option<String>,
}

#[derive(Debug)]
struct Runtime {
    account_id: Option<String>,
    api_key: Option<String>,
    base_url: String,
    environment: String,
    auth_source: &'static str,
    base_url_source: &'static str,
    fixture_mode: bool,
}

#[derive(Deserialize, Serialize)]
struct TokenCache {
    token: String,
    expires_at: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
    expires_in: Option<u64>,
    token_type: Option<String>,
    success: Option<bool>,
    message: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum ApprovalStatus {
    Pending,
    Approved,
    Used,
    Revoked,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ApprovalRequest {
    id: String,
    command: String,
    method: String,
    path: String,
    payload_hash: String,
    payload_redacted: Value,
    account_id: Option<String>,
    base_url: String,
    environment: String,
    created_at: u64,
    expires_at: u64,
    approved_at: Option<u64>,
    used_at: Option<u64>,
    status: ApprovalStatus,
    approval_token_hash: Option<String>,
}

struct CommandOutput {
    command: &'static str,
    data: Value,
    human: String,
    source: &'static str,
    extra: Map<String, Value>,
}

fn main() {
    let cli = Cli::parse();
    match execute(&cli) {
        Ok(output) => print_output(&cli, output),
        Err(error) => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "ok": false,
                        "error": {
                            "code": error.code,
                            "message": error.message,
                            "retryable": error.retryable
                        }
                    }))
                    .unwrap()
                );
            } else {
                eprintln!("{}: {}", error.code, error.message);
            }
            std::process::exit(1);
        }
    }
}

fn execute(cli: &Cli) -> AppResult<CommandOutput> {
    match &cli.command {
        Commands::Init(args) => init_config(args),
        _ => {
            let config = read_file_config()?;
            let runtime = build_runtime(cli, config);
            dispatch(cli, &runtime)
        }
    }
}

fn dispatch(cli: &Cli, runtime: &Runtime) -> AppResult<CommandOutput> {
    match &cli.command {
        Commands::Doctor => doctor(runtime),
        Commands::Init(_) => unreachable!(),
        Commands::Token { command } => match command {
            TokenCommands::Refresh => token_refresh(runtime),
        },
        Commands::Approvals { command } => match command {
            ApprovalCommands::List => approvals_list(),
            ApprovalCommands::Show(args) => approvals_show(&args.id),
            ApprovalCommands::Approve(args) => approvals_approve(args),
            ApprovalCommands::Revoke(args) => approvals_revoke(&args.id),
        },
        Commands::Account { command } => match command {
            AccountCommands::Get => {
                read_endpoint(runtime, "account.get", "/property", vec![], "account")
            }
        },
        Commands::Agents { command } => match command {
            AgentsCommands::List => {
                read_endpoint(runtime, "agents.list", "/agents", vec![], "agents")
            }
        },
        Commands::RoomTypes { command } => match command {
            RoomTypeCommands::List(args) => {
                let mut params = vec![];
                if args.show_photos {
                    params.push(("showPhotos".to_string(), "true".to_string()));
                }
                if let Some(portfolio_id) = &args.portfolio_id {
                    params.push(("portfolioId".to_string(), portfolio_id.clone()));
                }
                read_endpoint(
                    runtime,
                    "room-types.list",
                    "/room-types",
                    params,
                    "room_types",
                )
            }
            RoomTypeCommands::Get(args) => read_one(
                runtime,
                "room-types.get",
                &format!("/room-types/{}", encode_path(&args.id)),
                "room_types",
                &args.id,
                &["roomTypeId", "id"],
            ),
        },
        Commands::Rooms { command } => match command {
            RoomCommands::List => read_endpoint(runtime, "rooms.list", "/rooms", vec![], "rooms"),
            RoomCommands::Get(args) => read_one(
                runtime,
                "rooms.get",
                &format!("/rooms/{}", encode_path(&args.id)),
                "rooms",
                &args.id,
                &["roomId", "id"],
            ),
        },
        Commands::RatePlans { command } => match command {
            RatePlanCommands::List => read_endpoint(
                runtime,
                "rate-plans.list",
                "/rate-plans",
                vec![],
                "rate_plans",
            ),
        },
        Commands::Reservations { command } => match command {
            ReservationCommands::List(args) => reservations_list(runtime, args),
            ReservationCommands::Get(args) => read_one(
                runtime,
                "reservations.get",
                &format!("/reservations/{}", encode_path(&args.id)),
                "reservations",
                &args.id,
                &["reservationId", "id"],
            ),
            ReservationCommands::InHouse => read_endpoint(
                runtime,
                "reservations.in-house",
                "/reservations-inhouse",
                vec![],
                "reservations_inhouse",
            ),
        },
        Commands::Availability { command } => match command {
            AvailabilityCommands::Quote(args) => availability_quote(runtime, args),
        },
        Commands::Blocks { command } => match command {
            BlockCommands::List(args) => blocks_list(runtime, args),
        },
        Commands::Inventory { command } => match command {
            InventoryCommands::Get(args) => inventory_get(runtime, args),
        },
        Commands::Rates { command } => match command {
            RateCommands::Get(args) => rates_get(runtime, args),
            RateCommands::Preview(args) => {
                rates_preview(runtime, &args.rate_plan, &args.file, false)
            }
            RateCommands::Update(args) => rates_update(runtime, args),
        },
        Commands::Webhooks { command } => match command {
            WebhookCommands::List => {
                read_endpoint(runtime, "webhooks.list", "/webhooks", vec![], "webhooks")
            }
            WebhookCommands::Get(args) => read_one(
                runtime,
                "webhooks.get",
                &format!("/webhooks/{}", encode_path(&args.id)),
                "webhooks",
                &args.id,
                &["id", "webhookId"],
            ),
            WebhookCommands::Create(args) => webhook_create(runtime, args),
            WebhookCommands::Update(args) => webhook_update(runtime, args),
            WebhookCommands::Delete(args) => webhook_delete(runtime, args),
        },
        Commands::Request { command } => match command {
            RequestCommands::Get(args) => request_get(runtime, &args.path),
        },
    }
}

fn init_config(args: &InitArgs) -> AppResult<CommandOutput> {
    let base_url = args
        .base_url
        .clone()
        .unwrap_or_else(|| args.env.base_url().to_string());
    let config_dir = resly_dir()?;
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    let contents = format!(
        "account_id = {:?}\napi_key = {:?}\nbase_url = {:?}\nenvironment = {:?}\n",
        args.account_id,
        args.api_key,
        base_url,
        args.env.label()
    );
    fs::write(&config_path, contents)?;
    Ok(CommandOutput {
        command: "init",
        data: json!({
            "configPath": config_path,
            "environment": args.env.label(),
            "baseUrl": base_url,
            "accountId": args.account_id,
            "apiKey": redact(&args.api_key)
        }),
        human: format!(
            "Wrote {} for {} ({})",
            config_path.display(),
            args.env.label(),
            base_url
        ),
        source: "local",
        extra: Map::new(),
    })
}

fn doctor(runtime: &Runtime) -> AppResult<CommandOutput> {
    let config_path = config_path()?;
    let token_status = if runtime.fixture_mode {
        json!({
            "required": false,
            "available": false,
            "reason": "fixture mode does not require auth"
        })
    } else {
        match get_token(runtime) {
            Ok(token) => json!({
                "required": true,
                "available": true,
                "redacted": redact(&token)
            }),
            Err(error) => json!({
                "required": true,
                "available": false,
                "error": {
                    "code": error.code,
                    "message": error.message,
                    "retryable": error.retryable
                }
            }),
        }
    };
    Ok(CommandOutput {
        command: "doctor",
        data: json!({
            "version": env!("CARGO_PKG_VERSION"),
            "environment": runtime.environment,
            "baseUrl": runtime.base_url,
            "baseUrlSource": runtime.base_url_source,
            "auth": {
                "accountIdAvailable": runtime.account_id.is_some(),
                "apiKeyAvailable": runtime.api_key.is_some(),
                "source": runtime.auth_source
            },
            "fixtureMode": runtime.fixture_mode,
            "fixtureAvailable": fixture().is_ok(),
            "configPath": config_path,
            "token": token_status
        }),
        human: if runtime.fixture_mode {
            "Resly CLI is ready in fixture mode. Add RESLY_ACCOUNT_ID and RESLY_API_KEY or run resly init for live API use.".to_string()
        } else {
            format!(
                "Resly CLI is configured for {} at {}",
                runtime.environment, runtime.base_url
            )
        },
        source: if runtime.fixture_mode {
            "fixture"
        } else {
            "live"
        },
        extra: Map::new(),
    })
}

fn token_refresh(runtime: &Runtime) -> AppResult<CommandOutput> {
    if runtime.fixture_mode {
        return Ok(CommandOutput {
            command: "token.refresh",
            data: json!({
                "fixtureMode": true,
                "tokenRequired": false
            }),
            human: "Fixture mode does not require a token.".to_string(),
            source: "fixture",
            extra: Map::new(),
        });
    }
    let token = request_token(runtime)?;
    Ok(CommandOutput {
        command: "token.refresh",
        data: json!({
            "token": redact(&token),
            "baseUrl": runtime.base_url,
            "environment": runtime.environment
        }),
        human: format!(
            "Retrieved token {} for {}",
            redact(&token),
            runtime.environment
        ),
        source: "live",
        extra: Map::new(),
    })
}

fn approvals_list() -> AppResult<CommandOutput> {
    let mut approvals = Vec::new();
    let dir = approvals_dir()?;
    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            if let Ok(approval) = read_approval_path(&path) {
                approvals.push(approval);
            }
        }
    }
    approvals.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(CommandOutput {
        command: "approvals.list",
        data: json!({
            "approvals": approvals
                .iter()
                .map(|approval| approval_public_json(approval, false))
                .collect::<Vec<_>>()
        }),
        human: format!("Found {} local write approval request(s).", approvals.len()),
        source: "local",
        extra: Map::new(),
    })
}

fn approvals_show(id: &str) -> AppResult<CommandOutput> {
    let approval = read_approval(id)?;
    Ok(CommandOutput {
        command: "approvals.show",
        data: approval_public_json(&approval, true),
        human: format!(
            "{} {} is {}.",
            approval.method,
            approval.path,
            approval_status_label(&approval.status)
        ),
        source: "local",
        extra: Map::new(),
    })
}

fn approvals_approve(args: &ApprovalApproveArgs) -> AppResult<CommandOutput> {
    let mut approval = read_approval(&args.id)?;
    if approval.status != ApprovalStatus::Pending {
        return Err(AppError::new(
            "approval.invalid",
            format!(
                "Only pending approvals can be approved; {} is {}",
                approval.id,
                approval_status_label(&approval.status)
            ),
        ));
    }
    if approval.expires_at <= now_secs() {
        return Err(AppError::new(
            "approval.expired",
            format!(
                "Approval {} has expired; run the preview again",
                approval.id
            ),
        ));
    }
    let expected = format!("{} {}", approval.method, approval.path);
    if args.confirm_operation != expected {
        return Err(AppError::new(
            "approval.confirmation_mismatch",
            format!(
                "Expected --confirm-operation {:?} for approval {}",
                expected, approval.id
            ),
        ));
    }
    let token = format!("rat_{}", random_hex(24)?);
    approval.status = ApprovalStatus::Approved;
    approval.approved_at = Some(now_secs());
    approval.approval_token_hash = Some(hash_string(&token));
    write_approval(&approval)?;
    Ok(CommandOutput {
        command: "approvals.approve",
        data: json!({
            "id": approval.id,
            "status": approval.status,
            "approvalToken": token,
            "expiresAt": approval.expires_at,
            "operation": expected,
            "payloadHash": approval.payload_hash,
            "warning": "This token is shown once. Pass it with --approval-id and --approval-token on the matching live command."
        }),
        human: format!(
            "Approved {expected}. Token shown once: {token}. Use it only with --approval-id {}.",
            approval.id
        ),
        source: "local",
        extra: Map::new(),
    })
}

fn approvals_revoke(id: &str) -> AppResult<CommandOutput> {
    let mut approval = read_approval(id)?;
    if approval.status == ApprovalStatus::Used {
        return Err(AppError::new(
            "approval.invalid",
            format!("Approval {id} has already been used and cannot be revoked"),
        ));
    }
    approval.status = ApprovalStatus::Revoked;
    write_approval(&approval)?;
    Ok(CommandOutput {
        command: "approvals.revoke",
        data: approval_public_json(&approval, false),
        human: format!("Revoked approval {id}."),
        source: "local",
        extra: Map::new(),
    })
}

fn read_endpoint(
    runtime: &Runtime,
    command: &'static str,
    path: &str,
    params: Vec<(String, String)>,
    fixture_key: &str,
) -> AppResult<CommandOutput> {
    if runtime.fixture_mode {
        let data = fixture_value(fixture_key)?;
        return Ok(output(command, data, "fixture"));
    }
    let data = live_get(runtime, path, &params)?;
    Ok(output(command, data, "live"))
}

fn read_one(
    runtime: &Runtime,
    command: &'static str,
    path: &str,
    fixture_key: &str,
    id: &str,
    id_fields: &[&str],
) -> AppResult<CommandOutput> {
    if runtime.fixture_mode {
        let collection = fixture_value(fixture_key)?;
        let data = find_in_collection(collection, id, id_fields)?;
        return Ok(output(command, data, "fixture"));
    }
    let data = live_get(runtime, path, &[])?;
    Ok(output(command, data, "live"))
}

fn reservations_list(runtime: &Runtime, args: &ReservationListArgs) -> AppResult<CommandOutput> {
    let mut params = vec![(
        "dateType".to_string(),
        args.date_type.api_value().to_string(),
    )];
    match args.date_type {
        ReservationDateType::CheckIn | ReservationDateType::CheckOut => {
            let from = args.from.as_deref().ok_or_else(|| {
                AppError::new(
                    "input.missing",
                    "--from is required for check-in/check-out queries",
                )
            })?;
            let to = args.to.as_deref().ok_or_else(|| {
                AppError::new(
                    "input.missing",
                    "--to is required for check-in/check-out queries",
                )
            })?;
            validate_date(from, "--from")?;
            validate_date(to, "--to")?;
            params.push(("startDate".to_string(), from.to_string()));
            params.push(("endDate".to_string(), to.to_string()));
        }
        ReservationDateType::Updated => {
            let start_time = args.start_time.as_deref().ok_or_else(|| {
                AppError::new(
                    "input.missing",
                    "--start-time is required for updated queries",
                )
            })?;
            let end_time = args.end_time.as_deref().ok_or_else(|| {
                AppError::new(
                    "input.missing",
                    "--end-time is required for updated queries",
                )
            })?;
            params.push(("startTime".to_string(), start_time.to_string()));
            params.push(("endTime".to_string(), end_time.to_string()));
        }
    }
    params.push(("status".to_string(), args.status.clone()));
    if let Some(room_id) = &args.room_id {
        params.push(("roomId".to_string(), room_id.clone()));
    }

    let mut output = read_endpoint(
        runtime,
        "reservations.list",
        "/reservations",
        params,
        "reservations",
    )?;
    if let Some(items) = output.data.get_mut("data").and_then(Value::as_array_mut) {
        if items.len() > args.limit {
            items.truncate(args.limit);
        }
    }
    output
        .extra
        .insert("pagination".to_string(), json!({ "limit": args.limit }));
    Ok(output)
}

fn availability_quote(runtime: &Runtime, args: &AvailabilityQuoteArgs) -> AppResult<CommandOutput> {
    if args.guests == 0 {
        return Err(AppError::new(
            "input.invalid",
            "--guests must be greater than zero",
        ));
    }
    validate_date(&args.from, "--from")?;
    validate_date(&args.to, "--to")?;
    if args.from >= args.to {
        return Err(AppError::new(
            "date.invalid",
            "--to must be later than --from for availability quotes",
        ));
    }

    let room_types = collection_items(if runtime.fixture_mode {
        fixture_value("room_types")?
    } else {
        live_get(runtime, "/room-types", &[])?
    })?;
    let rate_plans = collection_items(if runtime.fixture_mode {
        fixture_value("rate_plans")?
    } else {
        live_get(runtime, "/rate-plans", &[])?
    })?;

    let mut options = Vec::new();
    let mut skipped_for_occupancy = 0_u64;
    for room_type in room_types {
        let max_occupancy = room_type
            .get("maxOccupancy")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if max_occupancy < args.guests {
            skipped_for_occupancy += 1;
            continue;
        }
        if room_type.get("active").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        let Some(room_type_id) = room_type.get("roomTypeId").and_then(Value::as_str) else {
            continue;
        };
        let rate_plan = rate_plans
            .iter()
            .find(|plan| {
                plan.get("roomTypeId").and_then(Value::as_str) == Some(room_type_id)
                    && plan.get("ratePlanId").and_then(Value::as_str).is_some()
            })
            .cloned();
        let rate_plan_id = rate_plan
            .as_ref()
            .and_then(|plan| plan.get("ratePlanId"))
            .and_then(Value::as_str);

        let inventory = inventory_value(runtime, room_type_id, &args.from, &args.to)?;
        let inventory_rows = collection_items(inventory)?;
        let stay_inventory: Vec<Value> = inventory_rows
            .into_iter()
            .filter(|row| {
                row.get("date")
                    .and_then(Value::as_str)
                    .map(|date| date >= args.from.as_str() && date < args.to.as_str())
                    .unwrap_or(false)
            })
            .collect();
        let min_available = stay_inventory
            .iter()
            .filter_map(|row| row.get("available").and_then(Value::as_i64))
            .min()
            .unwrap_or(0);

        let rates = if let Some(rate_plan_id) = rate_plan_id {
            rates_value(runtime, rate_plan_id, &args.from, &args.to)?
        } else {
            json!({ "data": [] })
        };
        let rate_rows = collection_items(rates)?;
        let stay_rates: Vec<Value> = rate_rows
            .into_iter()
            .filter(|row| {
                row.get("date")
                    .and_then(Value::as_str)
                    .map(|date| date >= args.from.as_str() && date < args.to.as_str())
                    .unwrap_or(false)
            })
            .collect();
        let total_rate: f64 = stay_rates
            .iter()
            .filter_map(|row| row.get("rate").and_then(Value::as_f64))
            .sum();
        let lowest_rate = stay_rates
            .iter()
            .filter_map(|row| row.get("rate").and_then(Value::as_f64))
            .min_by(|left, right| left.total_cmp(right));
        let stop_sell = stay_rates.iter().any(|row| {
            row.get("stopSell")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        });
        let available = min_available > 0 && !stop_sell && !stay_inventory.is_empty();
        let mut reasons = Vec::new();
        if stay_inventory.is_empty() {
            reasons.push("No inventory rows returned for the stay dates");
        }
        if min_available <= 0 {
            reasons.push("No rooms available on at least one night");
        }
        if stop_sell {
            reasons.push("Stop-sell is active on at least one night");
        }
        if rate_plan_id.is_none() {
            reasons.push("No rate plan found for this room type");
        }

        options.push(json!({
            "roomTypeId": room_type_id,
            "roomTypeName": room_type.get("roomTypeName").and_then(Value::as_str).unwrap_or(room_type_id),
            "maxOccupancy": max_occupancy,
            "ratePlanId": rate_plan_id,
            "currency": rate_plan
                .as_ref()
                .and_then(|plan| plan.get("currency"))
                .and_then(Value::as_str)
                .unwrap_or("AUD"),
            "available": available,
            "minAvailable": min_available.max(0),
            "nightsChecked": stay_inventory.len().max(stay_rates.len()),
            "lowestNightlyRate": lowest_rate,
            "estimatedTotal": if stay_rates.is_empty() { Value::Null } else { json!(total_rate) },
            "reasons": reasons
        }));
    }

    options.sort_by(|left, right| {
        let left_available = left
            .get("available")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let right_available = right
            .get("available")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        right_available
            .cmp(&left_available)
            .then_with(|| {
                option_total(left)
                    .partial_cmp(&option_total(right))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                left.get("roomTypeName")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .cmp(
                        right
                            .get("roomTypeName")
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                    )
            })
    });
    let available_count = options
        .iter()
        .filter(|option| {
            option
                .get("available")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let recommended = options
        .iter()
        .find(|option| {
            option
                .get("available")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .cloned()
        .unwrap_or(Value::Null);
    if options.len() > args.limit {
        options.truncate(args.limit);
    }

    Ok(CommandOutput {
        command: "availability.quote",
        data: json!({
            "guests": args.guests,
            "from": args.from,
            "to": args.to,
            "availableOptions": available_count,
            "skippedForOccupancy": skipped_for_occupancy,
            "recommended": recommended,
            "options": options
        }),
        human: format!(
            "Found {available_count} available option(s) for {} guest(s) from {} to {}.",
            args.guests, args.from, args.to
        ),
        source: if runtime.fixture_mode {
            "fixture"
        } else {
            "live"
        },
        extra: Map::new(),
    })
}

fn option_total(option: &Value) -> f64 {
    option
        .get("estimatedTotal")
        .and_then(Value::as_f64)
        .unwrap_or(f64::MAX)
}

fn inventory_get(runtime: &Runtime, args: &InventoryArgs) -> AppResult<CommandOutput> {
    validate_date(&args.from, "--from")?;
    validate_date(&args.to, "--to")?;
    Ok(output(
        "inventory.get",
        inventory_value(runtime, &args.room_type, &args.from, &args.to)?,
        if runtime.fixture_mode {
            "fixture"
        } else {
            "live"
        },
    ))
}

fn rates_get(runtime: &Runtime, args: &RateReadArgs) -> AppResult<CommandOutput> {
    validate_date(&args.from, "--from")?;
    validate_date(&args.to, "--to")?;
    Ok(output(
        "rates.get",
        rates_value(runtime, &args.rate_plan, &args.from, &args.to)?,
        if runtime.fixture_mode {
            "fixture"
        } else {
            "live"
        },
    ))
}

fn inventory_value(runtime: &Runtime, room_type: &str, from: &str, to: &str) -> AppResult<Value> {
    if runtime.fixture_mode {
        return fixture_scoped_value("inventory_by_room_type", "inventory", room_type, from, to);
    }
    live_get(
        runtime,
        &format!("/room-types/{}/inventory", encode_path(room_type)),
        &[
            ("startDate".to_string(), from.to_string()),
            ("endDate".to_string(), to.to_string()),
        ],
    )
}

fn rates_value(runtime: &Runtime, rate_plan: &str, from: &str, to: &str) -> AppResult<Value> {
    if runtime.fixture_mode {
        return fixture_scoped_value("rates_by_rate_plan", "rates", rate_plan, from, to);
    }
    live_get(
        runtime,
        &format!(
            "/rate-plans/{}/rates-and-restrictions",
            encode_path(rate_plan)
        ),
        &[
            ("startDate".to_string(), from.to_string()),
            ("endDate".to_string(), to.to_string()),
        ],
    )
}

fn blocks_list(runtime: &Runtime, args: &BlockListArgs) -> AppResult<CommandOutput> {
    validate_date(&args.from, "--from")?;
    validate_date(&args.to, "--to")?;
    let mut params = vec![
        (
            "dateType".to_string(),
            args.date_type.api_value().to_string(),
        ),
        ("startDate".to_string(), args.from.clone()),
        ("endDate".to_string(), args.to.clone()),
    ];
    if let Some(room_id) = &args.room_id {
        params.push(("roomId".to_string(), room_id.clone()));
    }
    read_endpoint(runtime, "blocks.list", "/blocks", params, "blocks")
}

fn rates_preview(
    runtime: &Runtime,
    rate_plan: &str,
    file: &Path,
    update_requested: bool,
) -> AppResult<CommandOutput> {
    let payload = read_rate_payload(file)?;
    let command = if update_requested {
        "rates.update"
    } else {
        "rates.preview"
    };
    let path = format!(
        "/rate-plans/{}/rates-and-restrictions",
        encode_path(rate_plan)
    );
    let approval = create_approval_request(runtime, "rates.update", "PATCH", &path, &payload)?;
    Ok(CommandOutput {
        command,
        data: json!({
            "ratePlanId": rate_plan,
            "endpoint": path,
            "method": "PATCH",
            "live": false,
            "fixtureMode": runtime.fixture_mode,
            "payload": payload,
            "approval": approval_public_json(&approval, false)
        }),
        human: format!(
            "Prepared {} rate/restriction update for {}. No live request was sent. Approval request: {}.",
            file.display(),
            rate_plan,
            approval.id
        ),
        source: "local",
        extra: Map::new(),
    })
}

fn rates_update(runtime: &Runtime, args: &RateUpdateArgs) -> AppResult<CommandOutput> {
    let payload = read_rate_payload(&args.file)?;
    let path = format!(
        "/rate-plans/{}/rates-and-restrictions",
        encode_path(&args.rate_plan)
    );
    if !args.live || runtime.fixture_mode {
        return rates_preview(runtime, &args.rate_plan, &args.file, true);
    }
    if args.test_only && runtime.base_url != DEFAULT_TEST_URL {
        return Err(AppError::new(
            "write.refused",
            "--test-only only permits writes against https://test.api.resly.com.au",
        ));
    }
    enforce_write_environment(
        runtime,
        args.confirm_account.as_deref(),
        args.confirm_environment.as_deref(),
    )?;
    let approval = require_approved_operation(
        runtime,
        "rates.update",
        "PATCH",
        &path,
        &payload,
        args.approval_id.as_deref(),
        args.approval_token.as_deref(),
    )?;
    let data = live_patch(runtime, &path, &payload)?;
    mark_approval_used(&approval.id)?;
    Ok(output("rates.update", data, "live"))
}

fn webhook_create(runtime: &Runtime, args: &WebhookCreateArgs) -> AppResult<CommandOutput> {
    let mut payload = json!({
        "url": args.url,
        "hookType": args.hook_type.api_value()
    });
    if let Some(username) = &args.basic_auth_username {
        payload["basicAuthUsername"] = json!(username);
    }
    if let Some(password) = &args.basic_auth_password {
        payload["basicAuthPassword"] = json!(password);
    }
    write_or_preview(
        runtime,
        "webhooks.create",
        "POST",
        "/webhooks",
        payload,
        args.live && !args.dry_run,
        args.confirm_account.as_deref(),
        args.confirm_environment.as_deref(),
        args.approval_id.as_deref(),
        args.approval_token.as_deref(),
    )
}

fn webhook_update(runtime: &Runtime, args: &WebhookUpdateArgs) -> AppResult<CommandOutput> {
    let mut payload = Map::new();
    if let Some(url) = &args.url {
        payload.insert("url".to_string(), json!(url));
    }
    if let Some(hook_type) = args.hook_type {
        payload.insert("hookType".to_string(), json!(hook_type.api_value()));
    }
    if payload.is_empty() {
        return Err(AppError::new(
            "input.missing",
            "Provide --url or --type for webhook update",
        ));
    }
    write_or_preview(
        runtime,
        "webhooks.update",
        "PATCH",
        &format!("/webhooks/{}", encode_path(&args.id)),
        Value::Object(payload),
        args.live && !args.dry_run,
        args.confirm_account.as_deref(),
        args.confirm_environment.as_deref(),
        args.approval_id.as_deref(),
        args.approval_token.as_deref(),
    )
}

fn webhook_delete(runtime: &Runtime, args: &WebhookDeleteArgs) -> AppResult<CommandOutput> {
    if args.live
        && !args.dry_run
        && !runtime.fixture_mode
        && args.confirm_delete.as_deref() != Some(&args.id)
    {
        return Err(AppError::new(
            "write.refused",
            "Live webhook deletes require --confirm-delete matching the webhook ID",
        ));
    }
    write_or_preview(
        runtime,
        "webhooks.delete",
        "DELETE",
        &format!("/webhooks/{}", encode_path(&args.id)),
        json!({}),
        args.live && !args.dry_run,
        args.confirm_account.as_deref(),
        args.confirm_environment.as_deref(),
        args.approval_id.as_deref(),
        args.approval_token.as_deref(),
    )
}

fn write_or_preview(
    runtime: &Runtime,
    command: &'static str,
    method: &'static str,
    path: &str,
    payload: Value,
    live: bool,
    confirm_account: Option<&str>,
    confirm_environment: Option<&str>,
    approval_id: Option<&str>,
    approval_token: Option<&str>,
) -> AppResult<CommandOutput> {
    if !live || runtime.fixture_mode {
        let approval = create_approval_request(runtime, command, method, path, &payload)?;
        return Ok(CommandOutput {
            command,
            data: json!({
                "method": method,
                "endpoint": path,
                "live": false,
                "fixtureMode": runtime.fixture_mode,
                "payload": redact_sensitive_json(payload),
                "approval": approval_public_json(&approval, false)
            }),
            human: format!(
                "Prepared {method} {path}. No live request was sent. Approval request: {}.",
                approval.id
            ),
            source: "local",
            extra: Map::new(),
        });
    }
    enforce_write_environment(runtime, confirm_account, confirm_environment)?;
    let approval = require_approved_operation(
        runtime,
        command,
        method,
        path,
        &payload,
        approval_id,
        approval_token,
    )?;
    let data = match method {
        "POST" => live_post(runtime, path, &payload)?,
        "PATCH" => live_patch(runtime, path, &payload)?,
        "DELETE" => live_delete(runtime, path)?,
        _ => return Err(AppError::new("method.unsupported", method)),
    };
    mark_approval_used(&approval.id)?;
    Ok(output(command, data, "live"))
}

fn request_get(runtime: &Runtime, raw_path: &str) -> AppResult<CommandOutput> {
    if runtime.fixture_mode {
        let (path, query) = split_raw_path(raw_path);
        if let Some(room_type) = scoped_path_id(path, "room-types", "inventory") {
            let from =
                query_param(query, &["startDate", "from"]).unwrap_or_else(|| "0000-00-00".into());
            let to = query_param(query, &["endDate", "to"]).unwrap_or_else(|| "9999-99-99".into());
            return Ok(output(
                "request.get",
                fixture_scoped_value(
                    "inventory_by_room_type",
                    "inventory",
                    &room_type,
                    &from,
                    &to,
                )?,
                "fixture",
            ));
        }
        if let Some(rate_plan) = scoped_path_id(path, "rate-plans", "rates-and-restrictions") {
            let from =
                query_param(query, &["startDate", "from"]).unwrap_or_else(|| "0000-00-00".into());
            let to = query_param(query, &["endDate", "to"]).unwrap_or_else(|| "9999-99-99".into());
            return Ok(output(
                "request.get",
                fixture_scoped_value("rates_by_rate_plan", "rates", &rate_plan, &from, &to)?,
                "fixture",
            ));
        }
        let key = match path {
            "/property" => "account",
            "/agents" => "agents",
            "/room-types" => "room_types",
            "/rooms" => "rooms",
            "/rate-plans" => "rate_plans",
            "/reservations" => "reservations",
            "/reservations-inhouse" => "reservations_inhouse",
            "/blocks" => "blocks",
            "/webhooks" => "webhooks",
            _ => {
                return Err(AppError::new(
                    "fixture.missing",
                    format!("No fixture is mapped for {raw_path}"),
                ));
            }
        };
        return Ok(output("request.get", fixture_value(key)?, "fixture"));
    }
    let data = live_get(runtime, raw_path, &[])?;
    Ok(output("request.get", data, "live"))
}

fn split_raw_path(raw_path: &str) -> (&str, Option<&str>) {
    raw_path
        .split_once('?')
        .map(|(path, query)| (path, Some(query)))
        .unwrap_or((raw_path, None))
}

fn scoped_path_id(path: &str, collection: &str, child: &str) -> Option<String> {
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    match parts.as_slice() {
        [path_collection, id, path_child]
            if *path_collection == collection && *path_child == child =>
        {
            Some(decode_path(id))
        }
        _ => None,
    }
}

fn decode_path(value: &str) -> String {
    urlencoding::decode(value)
        .map(|decoded| decoded.into_owned())
        .unwrap_or_else(|_| value.to_string())
}

fn query_param(query: Option<&str>, names: &[&str]) -> Option<String> {
    query.and_then(|query| {
        query.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            names
                .contains(&key)
                .then(|| decode_path(value.replace('+', " ").as_str()))
        })
    })
}

fn create_approval_request(
    runtime: &Runtime,
    command: &str,
    method: &str,
    path: &str,
    payload: &Value,
) -> AppResult<ApprovalRequest> {
    let payload_hash = payload_hash(payload)?;
    let now = now_secs();
    let approval = ApprovalRequest {
        id: format!("apr_{}_{}_{}", now, &payload_hash[..12], random_hex(4)?),
        command: command.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        payload_hash,
        payload_redacted: redact_sensitive_json(payload.clone()),
        account_id: runtime.account_id.clone(),
        base_url: runtime.base_url.clone(),
        environment: runtime.environment.clone(),
        created_at: now,
        expires_at: now + APPROVAL_EXPIRY_SECS,
        approved_at: None,
        used_at: None,
        status: ApprovalStatus::Pending,
        approval_token_hash: None,
    };
    write_approval(&approval)?;
    Ok(approval)
}

fn require_approved_operation(
    runtime: &Runtime,
    command: &str,
    method: &str,
    path: &str,
    payload: &Value,
    approval_id: Option<&str>,
    approval_token: Option<&str>,
) -> AppResult<ApprovalRequest> {
    let approval_id = approval_id.ok_or_else(|| {
        AppError::new(
            "approval.required",
            "Live writes require --approval-id from a matching preview",
        )
    })?;
    let approval_token = approval_token.ok_or_else(|| {
        AppError::new(
            "approval.required",
            "Live writes require --approval-token from resly approvals approve",
        )
    })?;
    let approval = read_approval(approval_id)?;
    if approval.status != ApprovalStatus::Approved {
        return Err(AppError::new(
            "approval.invalid",
            format!(
                "Approval {} is {}; approve a fresh preview first",
                approval.id,
                approval_status_label(&approval.status)
            ),
        ));
    }
    if approval.expires_at <= now_secs() {
        return Err(AppError::new(
            "approval.expired",
            format!(
                "Approval {} has expired; run the preview again",
                approval.id
            ),
        ));
    }
    if approval.command != command || approval.method != method || approval.path != path {
        return Err(AppError::new(
            "approval.mismatch",
            "Approval does not match this command, method, or endpoint",
        ));
    }
    if approval.account_id != runtime.account_id
        || approval.base_url != runtime.base_url
        || approval.environment != runtime.environment
    {
        return Err(AppError::new(
            "approval.environment_mismatch",
            "Approval was created for a different account, base URL, or environment",
        ));
    }
    if approval.payload_hash != payload_hash(payload)? {
        return Err(AppError::new(
            "approval.payload_mismatch",
            "Approval payload hash does not match the payload being applied",
        ));
    }
    let expected_hash = approval.approval_token_hash.as_deref().ok_or_else(|| {
        AppError::new(
            "approval.invalid",
            "Approval is missing its token hash; approve a fresh preview",
        )
    })?;
    if hash_string(approval_token) != expected_hash {
        return Err(AppError::new(
            "approval.token_mismatch",
            "Approval token does not match this approval request",
        ));
    }
    Ok(approval)
}

fn enforce_write_environment(
    runtime: &Runtime,
    confirm_account: Option<&str>,
    confirm_environment: Option<&str>,
) -> AppResult<()> {
    if runtime.environment != "production" {
        return Ok(());
    }
    if env::var("RESLY_ALLOW_PRODUCTION_WRITES").ok().as_deref() != Some("1") {
        return Err(AppError::new(
            "write.refused",
            "Production writes require RESLY_ALLOW_PRODUCTION_WRITES=1",
        ));
    }
    let account_id = runtime
        .account_id
        .as_deref()
        .ok_or_else(|| AppError::new("auth.missing", "Production writes require an account ID"))?;
    if confirm_account != Some(account_id) {
        return Err(AppError::new(
            "write.refused",
            "Production writes require --confirm-account matching the configured account ID",
        ));
    }
    if confirm_environment != Some("production") {
        return Err(AppError::new(
            "write.refused",
            "Production writes require --confirm-environment production",
        ));
    }
    Ok(())
}

fn mark_approval_used(id: &str) -> AppResult<()> {
    let mut approval = read_approval(id)?;
    approval.status = ApprovalStatus::Used;
    approval.used_at = Some(now_secs());
    write_approval(&approval)
}

fn approval_public_json(approval: &ApprovalRequest, include_payload: bool) -> Value {
    let mut data = json!({
        "id": approval.id,
        "status": approval.status,
        "command": approval.command,
        "method": approval.method,
        "endpoint": approval.path,
        "operation": format!("{} {}", approval.method, approval.path),
        "payloadHash": approval.payload_hash,
        "accountId": approval.account_id,
        "baseUrl": approval.base_url,
        "environment": approval.environment,
        "createdAt": approval.created_at,
        "expiresAt": approval.expires_at,
        "approvedAt": approval.approved_at,
        "usedAt": approval.used_at,
        "approveCommand": format!(
            "resly approvals approve {} --confirm-operation '{} {}'",
            approval.id, approval.method, approval.path
        )
    });
    if include_payload {
        data["payloadRedacted"] = approval.payload_redacted.clone();
    }
    data
}

fn approval_status_label(status: &ApprovalStatus) -> &'static str {
    match status {
        ApprovalStatus::Pending => "pending",
        ApprovalStatus::Approved => "approved",
        ApprovalStatus::Used => "used",
        ApprovalStatus::Revoked => "revoked",
    }
}

fn read_approval(id: &str) -> AppResult<ApprovalRequest> {
    if !id
        .chars()
        .all(|value| value.is_ascii_alphanumeric() || value == '_' || value == '-')
    {
        return Err(AppError::new(
            "approval.invalid",
            "Approval IDs may contain only ASCII letters, numbers, '_' and '-'",
        ));
    }
    read_approval_path(&approval_path(id)?)
}

fn read_approval_path(path: &Path) -> AppResult<ApprovalRequest> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn write_approval(approval: &ApprovalRequest) -> AppResult<()> {
    let dir = approvals_dir()?;
    fs::create_dir_all(&dir)?;
    fs::write(
        approval_path(&approval.id)?,
        serde_json::to_string_pretty(approval)?,
    )?;
    Ok(())
}

fn approval_path(id: &str) -> AppResult<PathBuf> {
    Ok(approvals_dir()?.join(format!("{id}.json")))
}

fn approvals_dir() -> AppResult<PathBuf> {
    Ok(resly_dir()?.join("approvals"))
}

fn read_rate_payload(file: &Path) -> AppResult<Value> {
    let payload: Value = serde_json::from_str(&fs::read_to_string(file)?)?;
    let restrictions = payload
        .get("restrictions")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::new("payload.invalid", "Rate update requires restrictions[]"))?;
    if restrictions.is_empty() {
        return Err(AppError::new(
            "payload.invalid",
            "Rate update requires at least one restriction",
        ));
    }
    for item in restrictions {
        let date = item
            .get("date")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::new("payload.invalid", "Each restriction requires date"))?;
        validate_date(date, "restriction.date")?;
    }
    if payload.get("echoToken").and_then(Value::as_str).is_none() {
        return Err(AppError::new(
            "payload.invalid",
            "Rate update requires echoToken",
        ));
    }
    Ok(payload)
}

fn live_get(runtime: &Runtime, path: &str, params: &[(String, String)]) -> AppResult<Value> {
    let token = get_token(runtime)?;
    let client = Client::new();
    let url = absolute_url(runtime, path);
    let response = client.get(url).bearer_auth(token).query(params).send()?;
    parse_response(response)
}

fn live_post(runtime: &Runtime, path: &str, payload: &Value) -> AppResult<Value> {
    let token = get_token(runtime)?;
    let client = Client::new();
    let response = client
        .post(absolute_url(runtime, path))
        .bearer_auth(token)
        .json(payload)
        .send()?;
    parse_response(response)
}

fn live_patch(runtime: &Runtime, path: &str, payload: &Value) -> AppResult<Value> {
    let token = get_token(runtime)?;
    let client = Client::new();
    let response = client
        .patch(absolute_url(runtime, path))
        .bearer_auth(token)
        .json(payload)
        .send()?;
    parse_response(response)
}

fn live_delete(runtime: &Runtime, path: &str) -> AppResult<Value> {
    let token = get_token(runtime)?;
    let client = Client::new();
    let response = client
        .delete(absolute_url(runtime, path))
        .bearer_auth(token)
        .send()?;
    parse_response(response)
}

fn parse_response(response: reqwest::blocking::Response) -> AppResult<Value> {
    let status = response.status();
    let body = response.text()?;
    let parsed = serde_json::from_str::<Value>(&body).unwrap_or_else(|_| json!({ "body": body }));
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(AppError::new(
            "api.error",
            format!(
                "Resly API returned {status}: {}",
                redact_sensitive_string(&parsed.to_string())
            ),
        ))
    }
}

fn get_token(runtime: &Runtime) -> AppResult<String> {
    if let Ok(token) = env::var("RESLY_ACCESS_TOKEN").or_else(|_| env::var("RESLY_TOKEN")) {
        if !token.trim().is_empty() {
            return Ok(token);
        }
    }
    if let Ok(cache) = read_token_cache() {
        if cache.expires_at > now_secs() + 60 {
            return Ok(cache.token);
        }
    }
    request_token(runtime)
}

fn request_token(runtime: &Runtime) -> AppResult<String> {
    let account_id = runtime.account_id.as_deref().ok_or_else(|| {
        AppError::new(
            "auth.missing",
            "Missing RESLY_ACCOUNT_ID or account_id in ~/.resly/config.toml",
        )
    })?;
    let api_key = runtime.api_key.as_deref().ok_or_else(|| {
        AppError::new(
            "auth.missing",
            "Missing RESLY_API_KEY or api_key in ~/.resly/config.toml",
        )
    })?;
    let client = Client::new();
    let response = client
        .post(absolute_url(runtime, "/token"))
        .json(&json!({
            "accountId": account_id,
            "key": api_key
        }))
        .send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        return Err(AppError::new(
            "auth.failed",
            format!(
                "Token request returned {status}: {}",
                redact_sensitive_string(&body)
            ),
        ));
    }
    let token_response: TokenResponse = serde_json::from_str(&body)?;
    if token_response.success == Some(false) {
        return Err(AppError::new(
            "auth.failed",
            token_response
                .message
                .unwrap_or_else(|| "Token request was not successful".to_string()),
        ));
    }
    let token = token_response.token;
    let expires_in = token_response.expires_in.unwrap_or(86_400);
    write_token_cache(&TokenCache {
        token: token.clone(),
        expires_at: now_secs() + expires_in,
    })?;
    let _ = token_response.token_type;
    Ok(token)
}

fn read_file_config() -> AppResult<FileConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(FileConfig::default());
    }
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

fn build_runtime(cli: &Cli, file_config: FileConfig) -> Runtime {
    let account_id = cli
        .account_id
        .clone()
        .or_else(|| file_config.account_id.clone());
    let api_key = cli.api_key.clone().or_else(|| file_config.api_key.clone());
    let auth_source = if cli.account_id.is_some() || cli.api_key.is_some() {
        "flag-or-env"
    } else if file_config.account_id.is_some() || file_config.api_key.is_some() {
        "config"
    } else {
        "missing"
    };

    let (base_url, base_url_source) = if let Some(base_url) = cli.base_url.clone() {
        (base_url, "flag-or-env")
    } else if let Some(base_url) = file_config.base_url.clone() {
        (base_url, "config")
    } else {
        (DEFAULT_TEST_URL.to_string(), "default")
    };
    let environment = file_config
        .environment
        .clone()
        .unwrap_or_else(|| infer_environment(&base_url).to_string());
    let fixture_mode = cli.fixture || account_id.is_none() || api_key.is_none();
    Runtime {
        account_id,
        api_key,
        base_url,
        environment,
        auth_source,
        base_url_source,
        fixture_mode,
    }
}

fn infer_environment(base_url: &str) -> &'static str {
    if base_url.contains("test.") {
        "test"
    } else if base_url.contains("api.resly.com.au") {
        "production"
    } else {
        "custom"
    }
}

fn output(command: &'static str, data: Value, source: &'static str) -> CommandOutput {
    let human = summarize(command, &data);
    CommandOutput {
        command,
        data,
        human,
        source,
        extra: Map::new(),
    }
}

fn print_output(cli: &Cli, output: CommandOutput) {
    if cli.json {
        let runtime = read_file_config()
            .ok()
            .map(|config| build_runtime(cli, config))
            .unwrap_or_else(|| Runtime {
                account_id: None,
                api_key: None,
                base_url: DEFAULT_TEST_URL.to_string(),
                environment: "test".to_string(),
                auth_source: "missing",
                base_url_source: "default",
                fixture_mode: true,
            });
        let mut envelope = Map::new();
        envelope.insert("ok".to_string(), json!(true));
        envelope.insert("command".to_string(), json!(output.command));
        envelope.insert("environment".to_string(), json!(runtime.environment));
        envelope.insert("source".to_string(), json!(output.source));
        envelope.insert("data".to_string(), output.data);
        for (key, value) in output.extra {
            envelope.insert(key, value);
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&Value::Object(envelope)).unwrap()
        );
    } else {
        println!("{}", output.human);
    }
}

fn summarize(command: &str, data: &Value) -> String {
    if let Some(count) = data.get("count").and_then(Value::as_u64) {
        return format!("{command}: {count} item(s)");
    }
    if let Some(data_array) = data.get("data").and_then(Value::as_array) {
        return format!("{command}: {} item(s)", data_array.len());
    }
    if let Some(data_object) = data.get("data").and_then(Value::as_object) {
        if let Some(name) = data_object
            .get("name")
            .or_else(|| data_object.get("roomName"))
            .or_else(|| data_object.get("roomTypeName"))
            .and_then(Value::as_str)
        {
            return format!("{command}: {name}");
        }
    }
    format!("{command}: ok")
}

fn fixture() -> AppResult<Value> {
    Ok(serde_json::from_str(FIXTURE_JSON)?)
}

fn fixture_value(key: &str) -> AppResult<Value> {
    fixture()?
        .get(key)
        .cloned()
        .ok_or_else(|| AppError::new("fixture.missing", format!("Missing fixture key {key}")))
}

fn fixture_scoped_value(
    scoped_key: &str,
    fallback_key: &str,
    id: &str,
    from: &str,
    to: &str,
) -> AppResult<Value> {
    let data = fixture()?;
    let mut value = data
        .get(scoped_key)
        .and_then(|group| group.get(id))
        .cloned()
        .or_else(|| data.get(fallback_key).cloned())
        .ok_or_else(|| AppError::new("fixture.missing", format!("Missing fixture for {id}")))?;
    if let Some(items) = value.get_mut("data").and_then(Value::as_array_mut) {
        items.retain(|item| {
            item.get("date")
                .and_then(Value::as_str)
                .map(|date| date >= from && date <= to)
                .unwrap_or(true)
        });
        value["count"] = json!(items.len());
    }
    Ok(value)
}

fn collection_items(value: Value) -> AppResult<Vec<Value>> {
    value
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| AppError::new("fixture.invalid", "Collection response has no data[]"))
}

fn find_in_collection(collection: Value, id: &str, id_fields: &[&str]) -> AppResult<Value> {
    let items = collection
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::new("fixture.invalid", "Fixture collection has no data[]"))?;
    for item in items {
        for field in id_fields {
            if item.get(*field).and_then(Value::as_str) == Some(id) {
                return Ok(json!({
                    "type": collection.get("type").cloned().unwrap_or_else(|| json!("object")),
                    "data": item
                }));
            }
        }
    }
    Err(AppError::new(
        "not.found",
        format!("{id} was not found in fixture data"),
    ))
}

fn validate_date(value: &str, label: &str) -> AppResult<()> {
    let bytes = value.as_bytes();
    let ok = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit());
    if ok {
        Ok(())
    } else {
        Err(AppError::new(
            "date.invalid",
            format!("{label} must be formatted as YYYY-MM-DD"),
        ))
    }
}

fn absolute_url(runtime: &Runtime, path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }
    let base = runtime.base_url.trim_end_matches('/');
    if path.starts_with('/') {
        format!("{base}{path}")
    } else {
        format!("{base}/{path}")
    }
}

fn encode_path(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

fn resly_dir() -> AppResult<PathBuf> {
    let home = env::var("HOME").map_err(|_| AppError::new("env.missing", "HOME is not set"))?;
    Ok(PathBuf::from(home).join(".resly"))
}

fn config_path() -> AppResult<PathBuf> {
    Ok(resly_dir()?.join("config.toml"))
}

fn token_cache_path() -> AppResult<PathBuf> {
    Ok(resly_dir()?.join("token-cache.json"))
}

fn read_token_cache() -> AppResult<TokenCache> {
    Ok(serde_json::from_str(&fs::read_to_string(
        token_cache_path()?,
    )?)?)
}

fn write_token_cache(cache: &TokenCache) -> AppResult<()> {
    fs::create_dir_all(resly_dir()?)?;
    fs::write(token_cache_path()?, serde_json::to_string_pretty(cache)?)?;
    Ok(())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn payload_hash(payload: &Value) -> AppResult<String> {
    Ok(hash_string(&serde_json::to_string(payload)?))
}

fn hash_string(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex_bytes(&hasher.finalize())
}

fn random_hex(bytes: usize) -> AppResult<String> {
    let mut buffer = vec![0_u8; bytes];
    let mut file = fs::File::open("/dev/urandom").map_err(|error| {
        AppError::new(
            "random.unavailable",
            format!("Could not read secure random bytes: {error}"),
        )
    })?;
    file.read_exact(&mut buffer).map_err(|error| {
        AppError::new(
            "random.unavailable",
            format!("Could not read secure random bytes: {error}"),
        )
    })?;
    Ok(hex_bytes(&buffer))
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn redact(value: &str) -> String {
    if value.len() <= 8 {
        return "********".to_string();
    }
    format!("{}...{}", &value[..4], &value[value.len() - 4..])
}

fn redact_sensitive_json(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    let redacted = if key.to_lowercase().contains("password")
                        || key.to_lowercase().contains("token")
                        || key.to_lowercase().contains("key")
                    {
                        value
                            .as_str()
                            .map(redact)
                            .map(Value::String)
                            .unwrap_or(value)
                    } else {
                        redact_sensitive_json(value)
                    };
                    (key, redacted)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_sensitive_json).collect()),
        other => other,
    }
}

fn redact_sensitive_string(value: &str) -> String {
    value
        .replace("api_key", "api_key_redacted")
        .replace("access_token", "access_token_redacted")
        .replace("Authorization", "Authorization-Redacted")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_runtime() -> Runtime {
        Runtime {
            account_id: None,
            api_key: None,
            base_url: DEFAULT_TEST_URL.to_string(),
            environment: "test".to_string(),
            auth_source: "missing",
            base_url_source: "default",
            fixture_mode: true,
        }
    }

    #[test]
    fn validates_dates() {
        assert!(validate_date("2026-07-01", "--from").is_ok());
        assert!(validate_date("2026/07/01", "--from").is_err());
    }

    #[test]
    fn fixture_has_core_resources() {
        let data = fixture().unwrap();
        for key in [
            "account",
            "agents",
            "room_types",
            "rooms",
            "rate_plans",
            "reservations",
            "inventory",
            "inventory_by_room_type",
            "rates",
            "rates_by_rate_plan",
            "webhooks",
        ] {
            assert!(data.get(key).is_some(), "missing {key}");
        }
    }

    #[test]
    fn fixture_inventory_and_rates_are_scoped() {
        let inventory = fixture_scoped_value(
            "inventory_by_room_type",
            "inventory",
            "2BR-OCEAN",
            "2026-07-05",
            "2026-07-07",
        )
        .unwrap();
        assert_eq!(inventory["roomTypeId"], "2BR-OCEAN");
        assert_eq!(inventory["count"], 3);

        let rates = fixture_scoped_value(
            "rates_by_rate_plan",
            "rates",
            "BAR-2BR-OCEAN",
            "2026-07-05",
            "2026-07-07",
        )
        .unwrap();
        assert_eq!(rates["ratePlanId"], "BAR-2BR-OCEAN");
        assert_eq!(rates["count"], 3);
    }

    #[test]
    fn raw_fixture_request_uses_scoped_inventory_and_rates() {
        let runtime = fixture_runtime();
        let inventory = request_get(
            &runtime,
            "/room-types/2BR-OCEAN/inventory?startDate=2026-07-05&endDate=2026-07-07",
        )
        .unwrap();
        assert_eq!(inventory.data["roomTypeId"], "2BR-OCEAN");

        let rates = request_get(
            &runtime,
            "/rate-plans/BAR-2BR-OCEAN/rates-and-restrictions?startDate=2026-07-05&endDate=2026-07-07",
        )
        .unwrap();
        assert_eq!(rates.data["ratePlanId"], "BAR-2BR-OCEAN");
    }

    #[test]
    fn availability_quote_answers_guest_count_question() {
        let runtime = fixture_runtime();
        let output = availability_quote(
            &runtime,
            &AvailabilityQuoteArgs {
                guests: 3,
                from: "2026-07-05".to_string(),
                to: "2026-07-07".to_string(),
                limit: 5,
            },
        )
        .unwrap();
        assert_eq!(output.command, "availability.quote");
        assert_eq!(output.data["availableOptions"], 2);
        assert_eq!(output.data["recommended"]["roomTypeId"], "1BR-GARDEN");
        assert_eq!(output.data["recommended"]["estimatedTotal"], 630.0);
        assert!(
            output.data["options"]
                .as_array()
                .unwrap()
                .iter()
                .any(|option| option["roomTypeId"] == "2BR-OCEAN")
        );

        let limited_output = availability_quote(
            &runtime,
            &AvailabilityQuoteArgs {
                guests: 3,
                from: "2026-07-05".to_string(),
                to: "2026-07-07".to_string(),
                limit: 1,
            },
        )
        .unwrap();
        assert_eq!(limited_output.data["availableOptions"], 2);
        assert_eq!(limited_output.data["options"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn redacts_short_values() {
        assert_eq!(redact("secret"), "********");
        assert_eq!(redact("1234567890"), "1234...7890");
    }
}
