use std::{env, fs::{OpenOptions, remove_file}, io::{BufRead, BufReader, prelude::*}, mem, path::{Path, PathBuf}};
use regex::Regex;
use log::info;
use structopt::StructOpt;
use anyhow::{anyhow, Result};
use chrono::naive::NaiveDate;

#[derive(StructOpt)]
#[structopt(name = "beancount-sort", about = "Sorts a beancount file.")]
struct Cli {
    // the path to the beancount file we want to sort
    #[structopt(short, long, parse(from_os_str), help = "Filepath which has to be sorted.")]
    file: PathBuf,
    // Comma-separated list of section names
    // sections: String,
    #[structopt(short, long, parse(from_os_str), help = "Where to write the sorted file?")]
    out: PathBuf,
    #[structopt(short, long, default_value = "0", help = "Leave the first n lines where they are. (e.g. for modline)")]
    skipn: usize,
    #[structopt(long, help = "Leave one empty line between each entry?")]
    spaces: bool,
}

const SECTIONS: [&str; 7] = ["Header", "Options", "Accounts", "Commodities", "Other Entries", "Prices", "Transactions"];
const NDECO: usize = 4; // number of "$" to use at section headings
const DECO: &str = "€";

/// The main Object that holds all information about a ledger file.
/// Is returned by the function [read_file]
#[derive(Debug)]
struct LedgerFile {
    path: PathBuf,
    file: std::fs::File,
    entries: Vec<Entry>
}
impl LedgerFile {
    fn write_ledger_file(self, path: &Path, spaces: &bool) -> Result<(), anyhow::Error> {
        // check if path exist
        // match for every entry type and append content to file
        if path.exists() {
            remove_file(path).unwrap()
        };
        let mut file = OpenOptions::new().create(true)
                                         .append(true)
                                         .open(path)
                                         .unwrap();
        for entry in self.entries {
            let output = entry.content;
            if let Err(e) = writeln!(file, "{}", &output) {
                return Err(anyhow!("Couldnt write to file: {}", e));
            };
            if *spaces {
                // insert empty line if "spaces" flag is given
                writeln!(file).unwrap()
            };
        };
        Ok(())
    }
}

/// The Entry type holds one entry in a beancount file.
#[derive(Debug, Clone)]
struct Entry {
    content: String,
    //#[derivative(Default(value = "NaiveDate::from_ymd(2021, 1, 1)"))]
    date: NaiveDate,
    entry_type: EntryType,
}

/// All possible types of entries in a beancount file. Used by [Entry]
#[derive(Debug, Clone)]
enum EntryType {
    Account,
    Option,
    Commodity,
    OtherEntry,
    Price,
    Transaction,
    Indented,
    Section,
    Header,
    Comment,
}

/// The type of a line. Returned by [get_line_type]
#[derive(Debug, Clone)]
enum Line {
    Date(NaiveDate),
    Option,
    Comment,
    Indent,
    Empty,
    Section,
}

/// Reads a file at a given Path. Returns a Result with either a [LedgerFile] or an Error
fn read_file(path: &Path) -> Result<LedgerFile, anyhow::Error> {
    let display = path.display();
    let ledger_file = LedgerFile {
        path: path.to_path_buf(),
        file: match std::fs::File::open(path) {
            Err(why) => panic!("Couldn't open file {}: {}", display, why),
            Ok(file) => file,
        },
        entries: Vec::new()
    };
    Ok(ledger_file)
}

/// Creates a backup of the original beancount file.
/// The new name is old_name_backup.old_extension
fn backup_file(path: &Path) -> Result<(), anyhow::Error> {
    let path_backup = path.with_file_name(format!("{}_backup.{}",
                                                  path.file_stem().unwrap().to_string_lossy(),
                                                  path.extension().unwrap().to_string_lossy()));
    match std::fs::copy(&path, &path_backup) {
        Err(why) => panic!("Couldn't backup file {}: {}", path.display(), why),
        Ok(file) => file,
    };
    println!("Backup done {:?} -> {:?}", &path, &path_backup);
    Ok(())
}

/// Identifies the [Line] type of a given [str].
fn get_line_type(line: &str, n: &usize) -> Result<Line, anyhow::Error> {
    let re_first = Regex::new(r"^(.*?) ").unwrap();
    let matches = re_first.captures(line);
    let first_thing = match matches {
        Some(m) => m.get(1).unwrap().as_str().to_string(),
        None => String::from("")
    };
    let re_date = Regex::new(r"^(\d{4}-[01]\d-[0-3]\d)").unwrap();
    let re_option = Regex::new(r"^(option)").unwrap();
    let re_comment = Regex::new(r"^(;+)").unwrap();
    let re_indented = Regex::new(r"(?m)(^ +)\S").unwrap();
    let re_empty = Regex::new(r"^.{0}$").unwrap();
    let re_section = Regex::new(format!("^;{}", DECO.repeat(NDECO)).as_str()).unwrap();
    if re_date.is_match(line) {
        Ok(Line::Date(NaiveDate::parse_from_str(&first_thing, "%Y-%m-%d").unwrap()))
    } else if re_option.is_match(line) {
        Ok(Line::Option)
    // section has to be tested before comment
    } else if re_section.is_match(line) {
        Ok(Line::Section)
    } else if re_comment.is_match(line) {
        Ok(Line::Comment)
    } else if re_indented.is_match(line) {
        Ok(Line::Indent)
    } else if re_empty.is_match(line) {
        Ok(Line::Empty)
    } else {
        Err(anyhow!("Can't define line {}: \"{}\"", n, line))
    }
}

/// Creates an [Entry] from a given string and a date.
fn construct_dated_entry(line: &str, date: NaiveDate) -> Result<Entry, anyhow::Error> {
    let re = Regex::new(r"^\d{4}-[01]\d-[0-3]\d (\w+|\*|!)").unwrap();
    let matches = re.captures(line);
    let directive_string = match matches {
        Some(m) => m.get(1).unwrap().as_str(),
        None => return Err(anyhow!("Couldn't finde entry type."))
    };
    let entry = match directive_string {
        "*" | "!" => Entry{content: line.to_owned(), date, entry_type: EntryType::Transaction},
        "commodity" => Entry{content: line.to_owned(), date, entry_type: EntryType::Commodity},
        "price" => Entry{content: line.to_owned(), date, entry_type: EntryType::Price},
        "open" => Entry{content: line.to_owned(), date, entry_type: EntryType::Account},
        _ => Entry{content: line.to_owned(), date, entry_type: EntryType::OtherEntry}
    };
    Ok(entry)
}

fn find_entries(mut ledger_file: LedgerFile, n_skip: usize) -> Result<LedgerFile, anyhow::Error> {
    let reader = BufReader::new(&ledger_file.file);
    let mut lines = reader.lines();
    let mut line_vec: Vec<(String, Line)> = Vec::new();
    for _i in 0..n_skip {
        let line: String = lines.next().unwrap().unwrap();
        let entry = Entry{content: line, date: NaiveDate::from_ymd(1990, 1, 1), entry_type: EntryType::Header};
        ledger_file.entries.push(entry)
    }

    for (mut nn, line) in lines.enumerate() {
        nn += 1;
        let n = nn + n_skip;
        let line: String = line.unwrap();
        let line_type: Line = get_line_type(&line, &n).unwrap();
        line_vec.push((line.clone(), line_type.clone()));
        let mut entry: Entry = match line_type {
            // If line has a date: create a dated entry
            Line::Date(d) => construct_dated_entry(&line, d).unwrap(),
            // If line is an option: create an entry with default date
            Line::Option => Entry{content: line.to_owned(), date: NaiveDate::from_ymd(1990, 1, 1), entry_type: EntryType::Option},
            // If line is a section heading: ignore it
            Line::Section => continue,
            // If line is a comment: create an entry with default date
            Line::Comment => Entry{content: line.to_owned(), date: NaiveDate::from_ymd(1990, 1, 1), entry_type: EntryType::Comment},
            // If line is an indented line: create an entry with default date
            Line::Indent => Entry{content: line.to_owned(), date: NaiveDate::from_ymd(1990, 1, 1), entry_type: EntryType::Indented},
            // If line is an indented line: ignore it
            Line::Empty => continue,
        };
        // If the line is a Comment then add it to the content of the previous Entry
        if !(n_skip == 0 && nn == 1) {
            if let EntryType::Comment = ledger_file.entries.last().unwrap().entry_type {
                let comment_entry = ledger_file.entries.pop().unwrap();
                entry.content = comment_entry.content + "\n" + &entry.content;
            }
        }
        // If the line is indented and the last entry was either a Transaction or a Commodity then add its content to the previous Entrys content
        if let EntryType::Indented = entry.entry_type {
            let last_entry = ledger_file.entries.pop().unwrap();
            // continue only if last line was a MultiLine-Entry
            if let EntryType::Transaction | EntryType::Commodity = last_entry.entry_type {
                let content_new = last_entry.content.to_owned() + "\n" + &entry.content;
                let new_entry = Entry{
                    content: content_new,
                    date: last_entry.date,
                    entry_type: last_entry.entry_type,
                };
                ledger_file.entries.push(new_entry);
            } else {
                // otherwise panic
                return Err(anyhow!("Misplaced indented line: Line {}\n\"{}\"", n, entry.content))
            };
        } else {
            ledger_file.entries.push(entry.clone())
        };
    };
    Ok(ledger_file)
}


fn get_section_variant(entry: &str) -> Result<EntryType, anyhow::Error> {
//["Header", "Accounts", "Options", "Commodities", "Other Entries", "Prices", "Transactions"]
    let entry_type = match entry {
        "Accounts" => EntryType::Account,
        "Options" => EntryType::Option,
        "Commodities" => EntryType::Commodity,
        "Other Entries" => EntryType::OtherEntry,
        "Prices" => EntryType::Price,
        "Transactions" => EntryType::Transaction,
        "Header" => EntryType::Header,
        _ => return Err(anyhow!("Not handled Section Type \"{}\"", entry))
    };
    Ok(entry_type)
}

/// Sorts a [Vec] of [Entry] by their date and their section
fn sort_entries(mut entries: Vec<Entry>) -> Result<Vec<Entry>, anyhow::Error> {
    entries.sort_by_key(|e| e.date);
    let mut sorted_entries: Vec<Entry> = Vec::new();
    let deco = DECO.repeat(NDECO);
    println!("{:?}", SECTIONS);
    for section in SECTIONS {
        // create a new entry with the section heading like:
        // ;€€€€€€€€€€€€€€€\n;€€€€Options€€€€\n;€€€€€€€€€€€€€€€
        if section != "Header" {
            let section_string: String = {";".to_string() + &deco.clone() + &DECO.repeat(section.len()) + &deco + "\n" +
                                ";" + &deco + section + &deco + "\n" +
                                ";" + &deco + &DECO.repeat(section.len()) + &deco};
            let section_entry = Entry{content: section_string,
                                      date: NaiveDate::from_ymd(1990, 1, 1),
                                      entry_type: EntryType::Section};
            sorted_entries.push(section_entry);
        }
        let section_variant = get_section_variant(section).unwrap();
        let entries_iter = entries.iter();
        entries_iter.filter(|e| mem::discriminant(&e.entry_type) == mem::discriminant(&section_variant))
                    .for_each(|entry| {
            sorted_entries.push(entry.to_owned())
        })
    }
    Ok(sorted_entries)
}

fn main () -> Result<()> {
    let args = Cli::from_args();
    let current_dir = env::current_dir();
    info!("Current directory is {:?}", current_dir);
    println!("Selected beancount file is {:?}", &args.file);
    backup_file(&args.file)?;
    let mut ledger_file = read_file(&args.file).unwrap();
    ledger_file = find_entries(ledger_file, args.skipn).unwrap();
    ledger_file.entries = sort_entries(ledger_file.entries).unwrap();
    ledger_file.write_ledger_file(&args.out, &args.spaces).unwrap();
    Ok(())
}


#[cfg(test)]
mod test {
    use std::mem::discriminant;

    use super::*;

    #[test]
    fn test_get_section_variant() {
        assert_eq!(discriminant(&get_section_variant("Header").unwrap()), discriminant(&EntryType::Header));
        assert_eq!(discriminant(&get_section_variant("Accounts").unwrap()), discriminant(&EntryType::Account));
        assert_eq!(discriminant(&get_section_variant("Options").unwrap()), discriminant(&EntryType::Option));
        assert_eq!(discriminant(&get_section_variant("Commodities").unwrap()), discriminant(&EntryType::Commodity));
        assert_eq!(discriminant(&get_section_variant("Other Entries").unwrap()), discriminant(&EntryType::OtherEntry));
        assert_eq!(discriminant(&get_section_variant("Prices").unwrap()), discriminant(&EntryType::Price));
        assert_eq!(discriminant(&get_section_variant("Transactions").unwrap()), discriminant(&EntryType::Transaction));
        assert!(get_section_variant("abcdefg").is_err());
    }
    #[test]
    fn test_sort_entries() {
        let entries = vec![
            Entry{content:"3".to_string(), date: NaiveDate::from_ymd(2021, 01, 01), entry_type: EntryType::Transaction},
            Entry{content:"1".to_string(), date: NaiveDate::from_ymd(2021, 01, 02), entry_type: EntryType::Option},
            Entry{content:"2".to_string(), date: NaiveDate::from_ymd(2021, 01, 03), entry_type: EntryType::Account},
        ];
        let mut sorted_entries_function = sort_entries(entries).unwrap();
        let sorted_entries_manual = [
            Entry{content:"1".to_string(), date: NaiveDate::from_ymd(2021, 01, 02), entry_type: EntryType::Option},
            Entry{content:"2".to_string(), date: NaiveDate::from_ymd(2021, 01, 03), entry_type: EntryType::Account},
            Entry{content:"3".to_string(), date: NaiveDate::from_ymd(2021, 01, 01), entry_type: EntryType::Transaction},
        ];
        let mut i = 0;
        while i < sorted_entries_function.len() {
            if mem::discriminant(&sorted_entries_function[i].entry_type) == mem::discriminant(&EntryType::Section) {
                let val = sorted_entries_function.remove(i);
                // your code here
            } else {
                i += 1;
            }
        }
        // const SECTIONS: [&str; 7] = ["Header", "Options", "Accounts", "Commodities", "Other Entries", "Prices", "Transactions"];
        assert_eq!(sorted_entries_function[0].content, sorted_entries_manual[0].content);
        assert_eq!(sorted_entries_function[1].content, sorted_entries_manual[1].content);
        assert_eq!(sorted_entries_function[2].content, sorted_entries_manual[2].content);
    }
}
