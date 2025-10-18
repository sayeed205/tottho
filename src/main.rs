//! Tottho - Modern Offline Database Management Tool
//! 
//! A powerful, offline-first database management application built with Rust and GPUI,
//! following Zed's architectural patterns for performance and extensibility.

use anyhow::{Context, Result};
use gpui::{App, Application};
use std::env;
use std::path::PathBuf;
use std::process;

// Import core modules
mod core;
mod ui;

use core::{TotthoApp, TotthoConfig, init_logging};

/// Command line arguments for Tottho
#[derive(Debug, Clone)]
pub struct Args {
    /// Configuration file path
    pub config_path: Option<PathBuf>,
    /// Log level override
    pub log_level: Option<String>,
    /// Disable session restoration
    pub no_restore: bool,
    /// Show version and exit
    pub version: bool,
    /// Show help and exit
    pub help: bool,
    /// Enable debug mode
    pub debug: bool,
    /// Workspace to open on startup
    pub workspace: Option<PathBuf>,
}

impl Args {
    /// Parse command line arguments
    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut parsed = Args {
            config_path: None,
            log_level: None,
            no_restore: false,
            version: false,
            help: false,
            debug: false,
            workspace: None,
        };

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--config" | "-c" => {
                    if i + 1 < args.len() {
                        parsed.config_path = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        eprintln!("Error: --config requires a path argument");
                        process::exit(1);
                    }
                }
                "--log-level" | "-l" => {
                    if i + 1 < args.len() {
                        parsed.log_level = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        eprintln!("Error: --log-level requires a level argument");
                        process::exit(1);
                    }
                }
                "--no-restore" => {
                    parsed.no_restore = true;
                    i += 1;
                }
                "--version" | "-v" => {
                    parsed.version = true;
                    i += 1;
                }
                "--help" | "-h" => {
                    parsed.help = true;
                    i += 1;
                }
                "--debug" | "-d" => {
                    parsed.debug = true;
                    i += 1;
                }
                "--workspace" | "-w" => {
                    if i + 1 < args.len() {
                        parsed.workspace = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        eprintln!("Error: --workspace requires a path argument");
                        process::exit(1);
                    }
                }
                arg if arg.starts_with('-') => {
                    eprintln!("Error: Unknown option: {}", arg);
                    eprintln!("Use --help for usage information");
                    process::exit(1);
                }
                _ => {
                    // Treat as workspace path if no workspace specified yet
                    if parsed.workspace.is_none() {
                        parsed.workspace = Some(PathBuf::from(&args[i]));
                    } else {
                        eprintln!("Error: Multiple workspace paths specified");
                        process::exit(1);
                    }
                    i += 1;
                }
            }
        }

        parsed
    }

    /// Show help message
    pub fn show_help() {
        println!("Tottho v{} - Modern Offline Database Management Tool", core::VERSION);
        println!();
        println!("USAGE:");
        println!("    tottho [OPTIONS] [WORKSPACE]");
        println!();
        println!("OPTIONS:");
        println!("    -c, --config <PATH>      Use custom configuration file");
        println!("    -l, --log-level <LEVEL>  Set log level (trace, debug, info, warn, error)");
        println!("    -w, --workspace <PATH>   Open specific workspace on startup");
        println!("    -d, --debug              Enable debug mode");
        println!("        --no-restore         Don't restore previous session");
        println!("    -v, --version            Show version information");
        println!("    -h, --help               Show this help message");
        println!();
        println!("EXAMPLES:");
        println!("    tottho                           # Start with default settings");
        println!("    tottho --config my-config.toml   # Use custom configuration");
        println!("    tottho --debug --no-restore      # Debug mode without session restore");
        println!("    tottho /path/to/workspace        # Open specific workspace");
    }

    /// Show version information
    pub fn show_version() {
        println!("Tottho v{}", core::VERSION);
        println!("Built with Rust and GPUI");
        println!("Following Zed's architectural patterns");
    }

    /// Convert to core::Args
    pub fn to_core_args(&self) -> core::Args {
        core::Args {
            config_path: self.config_path.clone(),
            log_level: self.log_level.clone(),
            no_restore: self.no_restore,
            version: self.version,
            help: self.help,
            debug: self.debug,
            workspace: self.workspace.clone(),
        }
    }
}

/// Load configuration with command line overrides
async fn load_configuration(args: &Args) -> Result<TotthoConfig> {
    // Determine config file path
    let config_path = if let Some(path) = &args.config_path {
        path.clone()
    } else {
        // Use default config path from app paths
        let app_paths = TotthoApp::init_paths()
            .context("Failed to initialize application paths")?;
        app_paths.config_dir.join("tottho.toml")
    };

    // Load base configuration
    let mut config = TotthoConfig::load_from_file(&config_path).await
        .with_context(|| format!("Failed to load configuration from {:?}", config_path))?;

    // Apply command line overrides
    if let Some(log_level) = &args.log_level {
        config.logging.level = log_level.clone();
    }

    if args.no_restore {
        config.startup.restore_session = false;
    }

    if args.debug {
        config.logging.level = "debug".to_string();
        config.logging.console_logging = true;
    }

    // Validate final configuration
    config.validate()
        .context("Configuration validation failed")?;

    Ok(config)
}

/// Initialize the application core with configuration
async fn initialize_app_with_config(
    config: TotthoConfig,
    args: core::Args,
) -> Result<()> {
    tracing::info!("Initializing Tottho core with configuration");
    
    // Initialize paths
    let paths = TotthoApp::init_paths()
        .context("Failed to initialize application paths")?;

    // Initialize enhanced logging with configuration
    TotthoApp::init_logging_with_config(&paths, &config.logging)
        .context("Failed to initialize enhanced logging")?;

    // Create and initialize core systems
    let core = core::TotthoCore::new();
    core.initialize().await
        .context("Failed to initialize core systems")?;

    // Apply configuration settings
    tracing::info!("Configuration applied: restore_session={}, debug_mode={}", 
        config.startup.restore_session, args.debug);

    // Apply workspace restoration if enabled
    if config.startup.restore_session && !args.no_restore {
        tracing::info!("Restoring previous session");
        // TODO: Implement session restoration
    }

    // Open specific workspace if provided
    if let Some(workspace_path) = &args.workspace {
        tracing::info!("Opening workspace: {:?}", workspace_path);
        // TODO: Implement workspace opening
    }

    tracing::info!("Tottho core initialization completed successfully");
    Ok(())
}

/// Initialize the application with proper error handling
async fn initialize_application(args: Args) -> Result<()> {
    // Handle version and help flags
    if args.version {
        Args::show_version();
        return Ok(());
    }

    if args.help {
        Args::show_help();
        return Ok(());
    }

    // Initialize basic logging first (will be replaced by proper logging later)
    init_logging().context("Failed to initialize basic logging")?;

    // Load configuration
    let config = load_configuration(&args).await
        .context("Failed to load application configuration")?;

    tracing::info!("Starting Tottho v{} with configuration loaded", core::VERSION);
    tracing::debug!("Command line arguments: {:?}", args);
    tracing::debug!("Final configuration: {:?}", config);

    // Create and run the GPUI application following Zed's pattern
    Application::new().run(move |cx: &mut App| {
        tracing::info!("Starting Tottho v{} with GPUI", core::VERSION);
        
        // Create the TotthoApp instance
        let app = TotthoApp::new(cx);
        
        // Log configuration application
        tracing::info!("Configuration applied: restore_session={}, debug_mode={}", 
            config.startup.restore_session, args.debug);

        // Apply workspace restoration if enabled
        if config.startup.restore_session && !args.no_restore {
            tracing::info!("Restoring previous session");
            // TODO: Implement session restoration
        }

        // Open specific workspace if provided
        if let Some(workspace_path) = &args.workspace {
            tracing::info!("Opening workspace: {:?}", workspace_path);
            // TODO: Implement workspace opening
        }

        // Note: Async initialization will be handled by the application lifecycle
        // For now, we just create the app and let GPUI manage the lifecycle
        tracing::info!("Core systems will be initialized by the application lifecycle");

        tracing::info!("Tottho application created and initialization scheduled");
    });

    Ok(())
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Use tokio runtime for async initialization
    let rt = tokio::runtime::Runtime::new()
        .context("Failed to create async runtime")?;

    // Run the initialization in the async runtime
    rt.block_on(async {
        if let Err(e) = initialize_application(args).await {
            eprintln!("Error: {}", e);
            
            // Print error chain for better debugging
            let mut source = e.source();
            while let Some(err) = source {
                eprintln!("  Caused by: {}", err);
                source = err.source();
            }
            
            process::exit(1);
        }
    });

    Ok(())
}
