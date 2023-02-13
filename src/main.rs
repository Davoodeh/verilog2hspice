use eframe::{
    egui::{CentralPanel, Context},
    App, Frame,
};
use egui_file::FileDialog;
use std::{
    ffi::OsStr,
    fs::{read_to_string, File},
    io::Write,
    path::PathBuf,
};

/// The assignment keyword for processes.
const AND_OR_CONSTANT_ARGS: Option<&str> = Some("WE, RE, CK");

/// Holds a list of options to change the behavior of the program.
#[derive(Default, Debug)]
struct Features {
    convert_ands: bool,
    convert_ors: bool,
    convert_nots: bool,
    /// If true, adds `AND_OR_CONSTANT_ARGS` to the end of ands and ors.
    add_and_or_constant_args: bool,
}

impl Features {
    pub fn convert(&self, path: &PathBuf, and_or_args_postfix: Option<&str>) -> Result<(), String> {
        let file_map_err = |i| format!("{} (file: {:?})", i, path);
        let raw = read_to_string(path).map_err(file_map_err)?;
        let mut file = raw
            .split("\n")
            .into_iter()
            .map(|i| i.to_owned())
            .collect::<Vec<String>>();

        if self.convert_ands {
            Self::convert_and_or(&mut file, "&", "AN2 AND", and_or_args_postfix)?;
        }
        if self.convert_ors {
            Self::convert_and_or(&mut file, "|", "OR2 OR", and_or_args_postfix)?;
        }
        if self.convert_nots {
            Self::convert_nots(&mut file)?;
        }

        // Self::_write_file(path, raw, Some(".bak")).map_err(file_map_err)?;
        Self::_write_file(path, file.join("\n"), Some(".new")).map_err(file_map_err)?;

        Ok(())
    }

    /// Take out `x`, `a` and `b` from a given line `assign x = a operator b`.
    fn extract_assignment<'a>(
        s: &'a str,
        operator: &'a str,
    ) -> Option<(&'a str, &'a str, &'a str)> {
        if let Some((_, assignment /* ="x = a o b;" */)) = s.split_once("assign") {
            if let Some((dest /* ="x" */, src /* ="a o b;" */)) = assignment.split_once("=") {
                if let Some((o1 /* ="a" */, rest /* ="a o b;" */)) = src.split_once(operator) {
                    if let Some((o2 /* ="b" */, _)) = rest.split_once(";") {
                        return Some((dest.trim(), o1.trim(), o2.trim()));
                    }
                }
            }
        }
        None
    }

    /// Return the indentation (` ` and `\t`) of a line.
    fn indent<'a>(s: &'a str) -> String {
        let mut indent = String::new();
        for i in s.chars() {
            match i {
                ' ' | '\t' => indent.push(i),
                _ => break,
            }
        }
        indent
    }

    /// Convert `assign x = a [operator] b;` to `[middle_part]_[number] (x, a, b [, args_postfix])`.
    pub fn convert_and_or(
        file: &mut Vec<String>,
        operator: &str,
        middle_part: &str,
        args_postfix: Option<&str>,
    ) -> Result<(), &'static str> {
        let mut gate_number = 0;
        for i in file {
            if let Some((dest, o1, o2)) = Self::extract_assignment(&i, operator) {
                gate_number += 1;
                *i = format!(
                    "{}{}_{} ({}, {}, {}",
                    Self::indent(&i),
                    middle_part,
                    gate_number,
                    dest,
                    o1,
                    o2,
                );
                if let Some(args_postfix) = args_postfix {
                    *i += ", ";
                    *i += args_postfix;
                }
                *i += ");";
            }
        }
        Ok(())
    }

    /// Replace `~x` usages (mid-line) with `Nx` after adding `IV INVL_[number] (Nx, x);`.
    pub fn convert_nots(file: &mut Vec<String>) -> Result<(), &'static str> {
        let mut gate_number = 0;
        let mut defined_symbols = Vec::<String>::new();
        for i in file {
            let mut symbols = Vec::<String>::new();

            // if true, chars will be added to the symbol till false
            let mut is_collecting = false;

            for c in i.chars() {
                // If found, add the definitions before the line (for each ~ add it to `symbols`)
                if c == '~' {
                    is_collecting = true;
                    symbols.push(String::new());
                    continue;
                }

                if is_collecting {
                    let last = symbols.len() - 1;
                    // [A-Za-z0-9_] + [\[\]\\] are valid naming chars based on
                    // people.ece.cornell.edu/land/courses/ece5760/Verilog/FreescaleVerilog.pdf +
                    // samples
                    match c {
                        'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '[' | ']' | '\\' => {
                            symbols[last].push(c)
                        }
                        _ => {
                            is_collecting = false;
                        }
                    }
                }
            }

            // If the line had any symbols, add the definitions and replace them.
            if symbols.len() != 0 {
                gate_number += 1;
            }
            for symbol in symbols {
                // TODO check if ~ symbol is only used for this purpose for the next line can
                // potentially mess up some code
                *i = i.replace("~", "N");

                if !defined_symbols.contains(&symbol) {
                    *i = format!(
                        "{}IV INVL_{} (N{}, {});\n{}",
                        Self::indent(&i),
                        gate_number,
                        symbol,
                        symbol,
                        i
                    );
                    defined_symbols.push(symbol);
                }
            }
        }
        Ok(())
    }

    /// Simply write to a file using with an optional extension.
    fn _write_file(path: &PathBuf, raw: String, ext: Option<&str>) -> Result<(), std::io::Error> {
        let path = {
            let mut t = path.clone().into_os_string();
            if let Some(ext) = ext {
                t.push(OsStr::new(ext));
            }
            t
        };
        let mut file = File::create(path)?;
        file.write_all(raw.as_bytes())?;
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct Main {
    /// Holds the opened file
    opened_file: Option<PathBuf>,
    /// Holds the dialog component for opening files
    _open_file_dialog: Option<FileDialog>,
    features: Features,
}

impl App for Main {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            // File dialog
            if (ui.button("Open...")).clicked() {
                let mut dialog = FileDialog::open_file(self.opened_file.clone());
                dialog.open();
                self._open_file_dialog = Some(dialog);
            }
            if let Some(dialog) = &mut self._open_file_dialog {
                if dialog.show(ctx).selected() {
                    if let Some(file) = dialog.path() {
                        self.opened_file = Some(file);
                    }
                }
            }

            // Run
            ui.group(|ui| {
                let (is_enabled, label, path) = match &self.opened_file {
                    Some(path) => (true, "Create .new file", Some(path)),
                    None => (false, "Select a file", None),
                };

                ui.set_enabled(is_enabled);
                if ui.button(label).clicked() {
                    self.features
                        .convert(
                            path.unwrap(), // `set_enabled` line protects this unwrap
                            if self.features.add_and_or_constant_args {
                                AND_OR_CONSTANT_ARGS
                            } else {
                                None
                            },
                        )
                        .unwrap(); // TODO handle gracefully
                }
            });

            // Feature
            ui.checkbox(&mut self.features.convert_nots, "Convert NOTs");
            ui.checkbox(&mut self.features.convert_ors, "Convert ORs");
            ui.checkbox(&mut self.features.convert_ands, "Convert ANDs");
            if let Some(v) = AND_OR_CONSTANT_ARGS {
                ui.checkbox(
                    &mut self.features.add_and_or_constant_args,
                    format!("Add constant {:?} arguments to ANDs and ORs", v),
                );
            }

            ui.label("more info on https://github.com/davoodeh/verilog2hspice");

            // Exit
            if ui.button("Exit").clicked() {
                _frame.close();
            }
        });
    }
}

fn main() {
    eframe::run_native(
        "Verilog2HSpice",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Box::new(Main::default())),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assignment_extraction() {
        assert_eq!(
            Features::extract_assignment(" assign x = a | b ; ", "|"),
            Some(("x", "a", "b")),
        );
        assert_eq!(
            Features::extract_assignment(" assign x = a | b ; \r", "|"),
            Some(("x", "a", "b")),
        );
        assert_eq!(
            Features::extract_assignment("assign x=a|b ;", "|"),
            Some(("x", "a", "b")),
        );
        assert_eq!(
            Features::extract_assignment(" assign x = a | b ; \r", "&"),
            None,
        );
    }
}
