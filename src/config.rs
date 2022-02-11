use std::env;

use clap::Parser;
use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use serde::Deserialize;
use xdg::BaseDirectories;

use crate::uninstall::Uninstall;

#[derive(Parser, Deserialize)]
#[clap(version = "0.1.0", author = "Danilo Spinella <oss@danyspin97.org>")]
pub struct Config {
    #[serde(skip_deserializing)]
    #[clap(short, long, help = "Path to the rinstall.yml configuration")]
    pub config: Option<String>,
    #[serde(skip_deserializing)]
    #[clap(long = "system", help = "Perform a system-wide installation")]
    pub system: bool,
    #[serde(skip_deserializing)]
    #[clap(
        short = 'y',
        long = "yes",
        help = "Accept the changes and perform the installation"
    )]
    pub accept_changes: bool,
    #[clap(
        short = 'f',
        long = "force",
        help = "Force the installation by overwriting (non-config) files",
        conflicts_with = "destdir"
    )]
    pub force: bool,
    #[clap(
        long = "update-config",
        help = "Overwrite the existing configurations of the package",
        conflicts_with = "destdir"
    )]
    pub update_config: bool,
    #[serde(skip_deserializing)]
    #[clap(
        short = 'P',
        long,
        help = "Path to the directory containing the project to install"
    )]
    pub package_dir: Option<String>,
    #[serde(skip_deserializing, default)]
    #[clap(
        short = 'p',
        long = "pkgs",
        help = "List of packages to install, separated by a comma"
    )]
    pub packages: Vec<String>,
    #[serde(skip_deserializing)]
    #[clap(long = "disable-uninstall")]
    pub disable_uninstall: bool,
    #[serde(skip_deserializing)]
    #[clap(short = 'D', long, requires = "system")]
    pub destdir: Option<String>,
    #[clap(long)]
    pub prefix: Option<String>,
    #[clap(long)]
    pub exec_prefix: Option<String>,
    #[clap(long)]
    pub bindir: Option<String>,
    #[clap(long, requires = "system")]
    pub sbindir: Option<String>,
    #[clap(long)]
    pub libdir: Option<String>,
    #[clap(long, requires = "system")]
    pub libexecdir: Option<String>,
    #[clap(long)]
    pub datarootdir: Option<String>,
    #[clap(long)]
    pub datadir: Option<String>,
    #[clap(long)]
    pub sysconfdir: Option<String>,
    #[clap(long)]
    pub localstatedir: Option<String>,
    #[clap(long)]
    pub runstatedir: Option<String>,
    #[clap(long, requires = "system")]
    pub includedir: Option<String>,
    #[clap(long, requires = "system")]
    pub docdir: Option<String>,
    #[clap(long, requires = "system")]
    pub mandir: Option<String>,
    #[clap(long, requires = "system")]
    pub pam_modulesdir: Option<String>,
    #[clap(long)]
    pub systemd_unitsdir: Option<String>,
    #[serde(skip_deserializing)]
    #[clap(
        long,
        help = "Use the generated binaries and libraries from the debug profile (only effective for rust projects)"
    )]
    pub rust_debug_target: bool,
    #[serde(skip_deserializing)]
    #[clap(subcommand)]
    pub subcmd: Option<SubCommand>,
}

#[derive(Parser, Clone)]
pub enum SubCommand {
    Uninstall(Uninstall),
    #[clap(name = "rpm-files")]
    GenerateRpmFiles,
}

macro_rules! merge_common_fields {
    ($update:expr, $other:expr) => {
        $update.config = $other.config;
        $update.system = $other.system;
        $update.accept_changes = $other.accept_changes;
        $update.force = $other.force;
        $update.update_config = $other.update_config;
        let current_dir = env::current_dir()
            .context("unable to get current directory")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        $update.package_dir = Some($other.package_dir.unwrap_or(current_dir));
        $update.packages = $other.packages;
        $update.disable_uninstall = $other.disable_uninstall;
        $update.destdir = $other.destdir;
        $update.rust_debug_target = $other.rust_debug_target;
        $update.subcmd = $other.subcmd;
    };
}
macro_rules! update_fields {
    ($update:expr, $other:expr, $($field:tt),*) => {
        $(
            if let Some($field) = $other.$field {
                $update.$field = Some($field);
            }
        )*
    };
}

impl Config {
    pub fn new_default_root() -> Self {
        Self {
            config: None,
            system: true,
            accept_changes: false,
            force: false,
            update_config: false,
            package_dir: None,
            packages: Vec::new(),
            disable_uninstall: false,
            destdir: None,
            prefix: Some("/usr/local".to_string()),
            exec_prefix: Some("@prefix@".to_string()),
            bindir: Some("@exec_prefix@/bin".to_string()),
            sbindir: Some("@exec_prefix@/sbin".to_string()),
            libdir: Some("@exec_prefix@/lib".to_string()),
            libexecdir: Some("@exec_prefix@/libexec".to_string()),
            datarootdir: Some("@prefix@/share".to_string()),
            datadir: Some("@prefix@/share".to_string()),
            sysconfdir: Some("@prefix@/etc".to_string()),
            localstatedir: Some("@prefix@/var".to_string()),
            runstatedir: Some("@localstatedir@/run".to_string()),
            includedir: Some("@prefix@/include".to_string()),
            docdir: Some("@datarootdir@/doc".to_string()),
            mandir: Some("@datarootdir@/man".to_string()),
            pam_modulesdir: Some("@libdir@/security".to_string()),
            systemd_unitsdir: Some("@libdir@/systemd".to_string()),
            rust_debug_target: false,
            subcmd: None,
        }
    }

    pub fn new_default_user() -> Self {
        Self {
            config: None,
            system: false,
            accept_changes: false,
            force: false,
            update_config: false,
            package_dir: None,
            packages: Vec::new(),
            disable_uninstall: false,
            destdir: None,
            prefix: None,
            exec_prefix: None,
            bindir: Some(".local/bin".to_string()),
            sbindir: None,
            libdir: Some(".local/lib".to_string()),
            libexecdir: Some(".local/libexec".to_string()),
            datarootdir: Some("@XDG_DATA_HOME@".to_string()),
            datadir: Some("@XDG_DATA_HOME@".to_string()),
            sysconfdir: Some("@XDG_CONFIG_HOME@".to_string()),
            localstatedir: Some("@XDG_DATA_HOME@".to_string()),
            runstatedir: Some("@XDG_RUNTIME_DIR@".to_string()),
            includedir: None,
            docdir: None,
            mandir: None,
            pam_modulesdir: None,
            systemd_unitsdir: Some("@sysconfdir@/systemd".to_string()),
            rust_debug_target: false,
            subcmd: None,
        }
    }

    pub fn merge_root_conf(
        &mut self,
        config: Self,
    ) {
        merge_common_fields!(self, config);

        update_fields!(
            self,
            config,
            prefix,
            exec_prefix,
            bindir,
            sbindir,
            libdir,
            libexecdir,
            datarootdir,
            datadir,
            sysconfdir,
            localstatedir,
            runstatedir,
            includedir,
            docdir,
            mandir,
            pam_modulesdir,
            systemd_unitsdir
        );
    }

    pub fn merge_user_conf(
        &mut self,
        config: Self,
    ) {
        merge_common_fields!(self, config);

        update_fields!(
            self,
            config,
            bindir,
            libdir,
            libexecdir,
            datarootdir,
            datadir,
            sysconfdir,
            localstatedir,
            runstatedir,
            systemd_unitsdir
        );
    }

    pub fn replace_user_placeholders(
        &mut self,
        xdg: &BaseDirectories,
    ) -> Result<()> {
        macro_rules! replace {
            ( $var:ident, $needle:literal, $replacement:expr ) => {
                self.$var = Some(self.$var.as_ref().unwrap().replace(
                    $needle,
                    $replacement.as_os_str().to_str().with_context(|| {
                        format!("unable to convert {:?} to String", $replacement)
                    })?,
                ));
            };
        }

        replace!(datarootdir, "@XDG_DATA_HOME@", xdg.get_data_home());
        replace!(datadir, "@XDG_DATA_HOME@", xdg.get_data_home());
        replace!(sysconfdir, "@XDG_CONFIG_HOME@", xdg.get_config_home());
        replace!(localstatedir, "@XDG_DATA_HOME@", xdg.get_data_home());
        let runtime_directory = xdg
            .get_runtime_directory()
            .context("insecure XDG_RUNTIME_DIR found")?;
        replace!(runstatedir, "@XDG_RUNTIME_DIR@", runtime_directory);
        replace!(systemd_unitsdir, "@XDG_CONFIG_HOME@", xdg.get_config_home());
        replace!(systemd_unitsdir, "@sysconfdir@", xdg.get_config_home());

        Ok(())
    }

    pub fn replace_root_placeholders(&mut self) {
        macro_rules! replace {
            ( $replacement:ident, $needle:literal, $($var:ident),* ) => {
                $(
                    self.$var = Some(self.$var
                        .as_ref()
                        .unwrap()
                        .replace($needle, self.$replacement.as_ref().unwrap()));
                )*
            };
        }

        replace!(
            prefix,
            "@prefix@",
            exec_prefix,
            bindir,
            sbindir,
            libdir,
            libexecdir,
            datadir,
            datarootdir,
            sysconfdir,
            localstatedir,
            runstatedir,
            includedir,
            docdir,
            mandir,
            pam_modulesdir,
            systemd_unitsdir
        );

        replace!(
            exec_prefix,
            "@exec_prefix@",
            bindir,
            sbindir,
            libdir,
            libexecdir
        );
        replace!(localstatedir, "@localstatedir@", runstatedir);
        replace!(datarootdir, "@datarootdir@", docdir, mandir);
        replace!(libdir, "@libdir@", pam_modulesdir, systemd_unitsdir);
    }
}
