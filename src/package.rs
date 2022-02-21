use std::path::{Path, PathBuf};

use color_eyre::{
    eyre::{ensure, Context, ContextCompat},
    Result,
};
use colored::Colorize;
use semver::{Version, VersionReq};
use serde::Deserialize;

use crate::icon::Icon;
use crate::install_entry::{string_or_struct, InstallEntry};
use crate::install_target::InstallTarget;
use crate::project::Project;
use crate::Dirs;

static PROJECTDIR_NEEDLE: &'static str = "$PROJECTDIR";

#[derive(Deserialize, Clone, PartialEq)]
pub enum Type {
    #[serde(rename(deserialize = "default"))]
    Default,
    #[serde(rename(deserialize = "rust"))]
    Rust,
    #[serde(rename(deserialize = "custom"))]
    Custom,
}

impl Default for Type {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Entry {
    #[serde(deserialize_with = "string_or_struct")]
    InstallEntry(InstallEntry),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum IconEntry {
    #[serde(deserialize_with = "string_or_struct")]
    Icon(Icon),
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Completions {
    #[serde(default)]
    pub bash: Vec<Entry>,
    #[serde(default)]
    pub fish: Vec<Entry>,
    #[serde(default)]
    pub zsh: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub name: Option<String>,
    #[serde(rename(deserialize = "type"), default)]
    pub project_type: Type,
    #[serde(default)]
    exe: Vec<Entry>,
    #[serde(default, rename(deserialize = "admin-exe"))]
    admin_exe: Vec<Entry>,
    #[serde(default)]
    libs: Vec<Entry>,
    #[serde(default)]
    libexec: Vec<Entry>,
    #[serde(default)]
    includes: Vec<Entry>,
    #[serde(default)]
    man: Vec<Entry>,
    #[serde(default)]
    data: Vec<Entry>,
    #[serde(default)]
    docs: Vec<Entry>,
    #[serde(default)]
    config: Vec<Entry>,
    #[serde(default, rename(deserialize = "user-config"))]
    user_config: Vec<Entry>,
    #[serde(default, rename(deserialize = "desktop-files"))]
    desktop_files: Vec<Entry>,
    #[serde(default, rename(deserialize = "appstream-metadata"))]
    appstream_metadata: Vec<Entry>,
    #[serde(default)]
    completions: Completions,
    #[serde(default, rename(deserialize = "pam-modules"))]
    pam_modules: Vec<Entry>,
    #[serde(default, rename(deserialize = "systemd-units"))]
    systemd_units: Vec<Entry>,
    #[serde(default, rename(deserialize = "systemd-user-units"))]
    systemd_user_units: Vec<Entry>,
    #[serde(default)]
    icons: Vec<IconEntry>,
    #[serde(default)]
    terminfo: Vec<Entry>,
    #[serde(default)]
    licenses: Vec<Entry>,
    #[serde(default, rename(deserialize = "pkg-config"))]
    pkg_config: Vec<Entry>,
}

macro_rules! entry {
    ( $x:ident ) => {
        match $x {
            Entry::InstallEntry(entry) => entry,
        }
    };
}

impl Package {
    pub fn targets(
        self,
        dirs: &Dirs,
        project: Project,
        rinstall_version: &Version,
        system_install: bool,
    ) -> Result<Vec<InstallTarget>> {
        let allowed_version = vec!["0.1.0"];
        allowed_version
            .iter()
            .map(|v| Version::parse(v).unwrap())
            .find(|v| v == rinstall_version)
            .with_context(|| format!("{} is not a valid rinstall version", rinstall_version))?;

        self.check_entries(rinstall_version)?;

        let package_name = self.name.unwrap();
        let mut results = Vec::new();

        macro_rules! get_files_impl {
            ( $files:tt, $install_dir:expr, $parent_dir:expr, $name:literal, $replace:literal ) => {
                self.$files
                    .into_iter()
                    .map(|entry| -> Result<InstallTarget> {
                        let entry = entry!(entry);
                        if $parent_dir == &project.outputdir
                            && entry.source.starts_with(PROJECTDIR_NEEDLE)
                        {
                            InstallTarget::new(
                                InstallEntry {
                                    source: entry
                                        .source
                                        .strip_prefix(PROJECTDIR_NEEDLE)?
                                        .to_path_buf(),
                                    destination: entry.destination,
                                    templating: entry.templating,
                                },
                                $install_dir,
                                &project.projectdir,
                                $replace,
                            )
                        } else {
                            InstallTarget::new(entry, $install_dir, $parent_dir, $replace)
                        }
                    })
                    .collect::<Result<Vec<InstallTarget>>>()
                    .with_context(|| format!("error while iterating {} files", $name))?
            };
        }
        macro_rules! get_files {
            ( $files:tt, $install_dir:expr, $parent_dir:expr, $name:literal ) => {
                get_files_impl!($files, $install_dir, $parent_dir, $name, true)
            };
        }
        macro_rules! get_no_replace_files {
            ( $files:tt, $install_dir:expr, $parent_dir:expr, $name:literal ) => {
                get_files_impl!($files, $install_dir, $parent_dir, $name, false)
            };
        }

        results.extend(get_files!(exe, &dirs.bindir, &project.outputdir, "exe"));
        if let Some(sbindir) = &dirs.sbindir {
            results.extend(get_files!(
                admin_exe,
                sbindir,
                &project.outputdir,
                "admin_exe"
            ));
        }
        results.extend(get_files!(libs, &dirs.libdir, &project.outputdir, "libs"));
        results.extend(get_files!(
            libexec,
            &dirs.libexecdir,
            &project.outputdir,
            "libexec"
        ));
        if let Some(includedir) = &dirs.includedir {
            results.extend(get_files!(
                includes,
                includedir,
                &project.projectdir,
                "includes"
            ));
        }
        results.extend(get_files!(
            data,
            &dirs.datadir.join(&package_name),
            &project.projectdir,
            "data"
        ));
        results.extend(get_no_replace_files!(
            config,
            &dirs.sysconfdir,
            &project.projectdir,
            "config"
        ));

        if let Some(mandir) = &dirs.mandir {
            results.extend(
                self.man
                    .into_iter()
                    .map(|entry| -> Result<InstallTarget> {
                        let Entry::InstallEntry(entry) = entry;
                        ensure!(
                            !entry
                                .source
                                .as_os_str()
                                .to_str()
                                .with_context(|| format!(
                                    "unable to convert {:?} to string",
                                    entry.source
                                ))?
                                .ends_with('/'),
                            "the man entry cannot be a directory"
                        );
                        let use_source_name = if let Some(destination) = &entry.destination {
                            destination
                                .as_os_str()
                                .to_str()
                                .with_context(|| {
                                    format!("unable to convert {:?} to string", entry.source)
                                })?
                                .ends_with('/')
                        } else {
                            true
                        };
                        let name = if use_source_name {
                            &entry.source
                        } else {
                            entry.destination.as_ref().unwrap()
                        };
                        let man_cat = name
                            .extension()
                            .with_context(|| format!("unable to get extension of file {:?}", name))?
                            .to_str()
                            .with_context(|| format!("unable to convert {:?} to string", name))?
                            .to_string();
                        ensure!(
                            man_cat.chars().next().unwrap().is_ascii_digit(),
                            "the last character should be a digit from 1 to 8"
                        );
                        let install_dir = mandir.join(format!("man{}", &man_cat));
                        InstallTarget::new(entry, &install_dir, &project.projectdir, true)
                    })
                    .collect::<Result<Vec<InstallTarget>>>()
                    .context("error while iterating man pages")?,
            );
        }

        if system_install {
            let pkg_docs = &dirs.docdir.as_ref().unwrap().join(Path::new(&package_name));
            results.extend(get_files!(docs, pkg_docs, &project.projectdir, "docs"));
            results.extend(get_files!(
                user_config,
                &pkg_docs.join("user-config"),
                &project.projectdir,
                "user-config"
            ));
        } else {
            results.extend(get_no_replace_files!(
                user_config,
                &dirs.sysconfdir,
                &project.projectdir,
                "user-config"
            ));
        }

        results.extend(get_files!(
            desktop_files,
            &dirs.datarootdir.join("applications"),
            &project.projectdir,
            "desktop-files"
        ));

        if system_install {
            results.extend(get_files!(
                appstream_metadata,
                &dirs.datarootdir.join("metainfo"),
                &project.projectdir,
                "appstream-metadata"
            ));
        }

        let mut completions = self
            .completions
            .bash
            .into_iter()
            .map(|completion| {
                (
                    completion,
                    if system_install {
                        "bash-completion/completions"
                    } else {
                        "bash-completion"
                    },
                )
            })
            .collect::<Vec<(Entry, &'static str)>>();
        if system_install {
            completions.extend(
                self.completions
                    .fish
                    .into_iter()
                    .map(|completion| (completion, "fish/vendor_completions.d")),
            );
            completions.extend(
                self.completions
                    .zsh
                    .into_iter()
                    .map(|completion| (completion, "zsh/site-functions")),
            );
        }
        results.extend(
            completions
                .into_iter()
                .map(|(entry, completionsdir)| -> Result<InstallTarget> {
                    InstallTarget::new(
                        entry!(entry),
                        &dirs.datarootdir.join(completionsdir),
                        &project.projectdir,
                        true,
                    )
                })
                .collect::<Result<Vec<InstallTarget>>>()
                .context("error while iterating completion files")?,
        );

        if let Some(pam_modulesdir) = &dirs.pam_modulesdir {
            results.extend(
                self.pam_modules
                    .into_iter()
                    .map(|entry| {
                        let Entry::InstallEntry(InstallEntry {
                            source,
                            destination,
                            templating,
                        }) = entry;

                        let destination = if destination.is_some() {
                            destination
                        } else {
                            let file_name = source
                                .file_name()
                                .with_context(|| {
                                    format!("unable to get file name of file {:?}", source)
                                })?
                                .to_str()
                                .unwrap();
                            if file_name.starts_with("libpam_") {
                                Some(PathBuf::from(file_name.strip_prefix("lib").unwrap()))
                            } else {
                                None
                            }
                        };

                        InstallTarget::new(
                            InstallEntry {
                                source,
                                destination,
                                templating,
                            },
                            pam_modulesdir,
                            &project.outputdir,
                            true,
                        )
                    })
                    .collect::<Result<Vec<InstallTarget>>>()
                    .context("error while iterating pam-modules")?,
            );
        }

        if system_install {
            results.extend(get_files!(
                systemd_units,
                &dirs.systemd_unitsdir.join("system"),
                &project.projectdir,
                "systemd-units"
            ));
        }
        results.extend(get_files!(
            systemd_user_units,
            &dirs.systemd_unitsdir.join("user"),
            &project.projectdir,
            "systemd-user-units"
        ));

        results.extend(
            self.icons
                .into_iter()
                .map(|icon| -> Icon {
                    match icon {
                        IconEntry::Icon(icon) => icon,
                    }
                })
                .filter(|icon| system_install || !icon.pixmaps)
                .map(|icon| -> Result<InstallTarget> {
                    InstallTarget::new(
                        InstallEntry {
                            source: icon.source.clone(),
                            destination: Some(icon.get_destination().with_context(|| {
                                format!(
                                    "unable to generate destination for icon {:?}",
                                    icon.source.clone()
                                )
                            })?),
                            templating: false,
                        },
                        &dirs.datarootdir,
                        &project.projectdir,
                        true,
                    )
                })
                .collect::<Result<Vec<InstallTarget>>>()
                .context("error while iterating icons")?,
        );

        if system_install {
            results.extend(
                self.terminfo
                    .into_iter()
                    .map(|entry| -> Result<InstallTarget> {
                        let Entry::InstallEntry(entry) = entry;
                        ensure!(
                            !entry
                                .source
                                .as_os_str()
                                .to_str()
                                .with_context(|| format!(
                                    "unable to convert {:?} to string",
                                    entry.source
                                ))?
                                .ends_with('/'),
                            "the terminfo entry cannot be a directory"
                        );
                        let use_source_name = if let Some(destination) = &entry.destination {
                            destination
                                .as_os_str()
                                .to_str()
                                .with_context(|| {
                                    format!("unable to convert {:?} to string", entry.source)
                                })?
                                .ends_with('/')
                        } else {
                            true
                        };
                        let name = if use_source_name {
                            &entry.source
                        } else {
                            entry.destination.as_ref().unwrap()
                        };
                        let initial = name
                            .file_name()
                            .with_context(|| format!("unable to get filename of file {:?}", name))?
                            .to_str()
                            .with_context(|| format!("unable to convert {:?} to string", name))?
                            .chars()
                            .next()
                            .with_context(|| {
                                format!("terminfo entry {:?} contains an empty filename", name)
                            })?
                            .to_lowercase()
                            .to_string();
                        let install_dir = dirs.datarootdir.join("terminfo").join(&initial);
                        InstallTarget::new(entry, &install_dir, &project.projectdir, true)
                    })
                    .collect::<Result<Vec<InstallTarget>>>()
                    .context("error while iterating terminfo files")?,
            );
        }

        results.extend(get_files!(
            licenses,
            &dirs.datarootdir.join("licenses").join(&package_name),
            &project.projectdir,
            "licenses"
        ));

        if system_install {
            results.extend(get_files!(
                pkg_config,
                &dirs.libdir.join("pkgconfig"),
                &project.projectdir,
                "pkg-config"
            ));
        }

        Ok(results)
    }

    fn check_entries(
        &self,
        rinstall_version: &Version,
    ) -> Result<()> {
        macro_rules! check_version_expr {
            ( $version:ident, $name:literal, $type:expr, $req:literal ) => {
                let requires = VersionReq::parse($req).unwrap();
                ensure!(
                    $type.is_empty() || requires.matches(&$version),
                    "{} requires version {}",
                    $name,
                    requires
                );
            };
        }
        macro_rules! check_version {
            ( $version:ident, $name:literal, $type:ident, $req:literal ) => {
                check_version_expr!($version, $name, self.$type, $req);
            };
        }

        if self.project_type == Type::Custom
            && VersionReq::parse(">=0.2.0")
                .unwrap()
                .matches(rinstall_version)
        {
            eprintln!(
                "{}: type '{}' has been deprecated, use '{}' or leave it empty",
                "WARNING".red().italic(),
                "custom".bright_black(),
                "default".bright_black(),
            );
        }
        check_version!(rinstall_version, "exe", exe, ">=0.1.0");
        check_version!(rinstall_version, "admin_exe", admin_exe, ">=0.1.0");
        check_version!(rinstall_version, "libs", libs, ">=0.1.0");
        check_version!(rinstall_version, "libexec", libexec, ">=0.1.0");
        check_version!(rinstall_version, "includes", includes, ">=0.1.0");
        check_version!(rinstall_version, "man", man, ">=0.1.0");
        check_version!(rinstall_version, "data", data, ">=0.1.0");
        check_version!(rinstall_version, "docs", docs, ">=0.1.0");
        check_version!(rinstall_version, "config", config, ">=0.1.0");
        check_version!(rinstall_version, "user-config", user_config, ">=0.1.0");
        check_version!(rinstall_version, "desktop-files", desktop_files, ">=0.1.0");
        check_version!(
            rinstall_version,
            "appstream-metadata",
            appstream_metadata,
            ">=0.1.0"
        );
        check_version_expr!(
            rinstall_version,
            "completions:bash",
            self.completions.bash,
            ">=0.1.0"
        );
        check_version_expr!(
            rinstall_version,
            "completions:fish",
            self.completions.fish,
            ">=0.1.0"
        );
        check_version_expr!(
            rinstall_version,
            "completions:zsh",
            self.completions.zsh,
            ">=0.1.0"
        );
        check_version!(rinstall_version, "pam-modules", pam_modules, ">=0.1.0");
        check_version!(rinstall_version, "systemd-units", systemd_units, ">=0.1.0");
        check_version!(rinstall_version, "icons", icons, ">=0.1.0");
        check_version!(rinstall_version, "terminfo", terminfo, ">=0.1.0");
        check_version!(rinstall_version, "licenses", licenses, ">=0.1.0");
        check_version!(rinstall_version, "pkg-config", pkg_config, ">=0.1.0");

        Ok(())
    }
}
