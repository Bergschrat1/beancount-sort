use std::{env, fs::{OpenOptions, remove_file}, io::{BufReader, BufRead}, mem, path::PathBuf};
use std::io::prelude::*;
use regex::Regex;
use log::info;
use structopt::StructOpt;
use anyhow::{anyhow, Result};
use chrono::naive::NaiveDate;

#[derive(StructOpt)]
#[structopt(name = "beancount_sort", about = "Sorts a beancount file.")]
struct Cli {
    // the path to the beancount file we want to sort
    #[structopt(short, long, parse(from_os_str))]
    path: PathBuf,
    // Comma-separated list of section names
    // sections: String,
    #[structopt(short, long, parse(from_os_str))]
    out: PathBuf,
    #[structopt(short, long, default_value = "0")]
    skipn: usize,
}

const SECTIONS: [&str; 7] = ["Header",  "Accounts", "Options", "Commodities", "Other Entries", "Prices", "Transactions"];

#[derive(Debug)]
struct LedgerFile {
    path: PathBuf,
    file: std::fs::File,
    entries: Vec<Entry>
}
impl LedgerFile {
    fn write_ledger_file(self, path: &PathBuf) -> Result<(), anyhow::Error> {
        // check if path exist
        // match for every entry type and append content to file
        // TODO Alternative: use SingleLineEntry and MultiLineEntry instead of all the Entry variants
        if path.exists() {
            remove_file(path).unwrap()
        };
        let mut file = OpenOptions::new().create(true)
                                         .append(true)
                                         .open(path)
                                         .unwrap();
        for entry in self.entries {
            let output = match entry {
                Entry::SingleLine(sle) => sle.content,
                Entry::MultiLine(mle) => mle.content,
            };
            if let Err(e) = writeln!(file, "{}", output) {
                return Err(anyhow!("Couldnt write to file: {}", e));
            }
        };
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct SingleLineEntry {
    content: String,
    //#[derivative(Default(value = "NaiveDate::from_ymd(2021, 1, 1)"))]
    date: NaiveDate,
    entry_type: EntryType,
}

#[derive(Debug, Clone)]
struct MultiLineEntry {
    content: String,
    //#[derivative(Default(value = "NaiveDate::from_ymd(2021, 1, 1)"))]
    date: NaiveDate,
    entry_type: EntryType,
}
#[derive(Debug, Clone)]
struct Account {
    content: String,
    //#[derivative(Default(value = "NaiveDate::from_ymd(2021, 1, 1)"))]
    date: NaiveDate
}

#[derive(Debug, Clone)]
struct Transaction {
    content: String,
    //#[derivative(Default(value = "NaiveDate::from_ymd(2021, 1, 1)"))]
    date: NaiveDate
}

#[derive(Debug, Clone)]
struct Price {
    content: String,
    //#[derivative(Default(value = "NaiveDate::from_ymd(2021, 1, 1)"))]
    date: NaiveDate
}

#[derive(Debug, Clone, Default)]
struct Option {
    content: String,
}

#[derive(Debug, Clone)]
struct Commodity {
    content: String,
    //#[derivative(Default(value = "NaiveDate::from_ymd(2021, 1, 1)"))]
    date: NaiveDate
}

#[derive(Debug, Clone)]
struct OtherEntry {
    content: String,
    //#[derivative(Default(value = "NaiveDate::from_ymd(2021, 1, 1)"))]
    date: NaiveDate
}

#[derive(Debug, Clone)]
enum EntryType {
    Account(),
    Option(),
    Commodity(),
    OtherEntry(),
    Price(),
    Transaction(),
    Indented(),
    Section(),
    Header(),
    Comment(),
}

#[derive(Debug, Clone)]
enum Entry {
    SingleLine(SingleLineEntry),
    MultiLine(MultiLineEntry),
}

#[derive(Debug, Clone)]
enum Line {
    Date(NaiveDate),
    Option,
    Comment,
    Indent,
    Empty,
}

fn read_file(path: &PathBuf) -> Result<LedgerFile, anyhow::Error> {
    let display = path.display();
    let ledger_file = LedgerFile {
        path: path.clone(),
        file: match std::fs::File::open(path) {
            Err(why) => panic!("Couldn't open file {}: {}", display, why),
            Ok(file) => file,
        },
        entries: Vec::new()
    };
    Ok(ledger_file)
}

fn backup_file(path: &PathBuf) -> Result<(), anyhow::Error> {
    let path_backup = path.with_file_name(format!("{}_backup.{}",
                                                  path.file_stem().unwrap().to_string_lossy(),
                                                  path.extension().unwrap().to_string_lossy()));
    let display = path.display();
    match std::fs::copy(&path, &path_backup) {
        Err(why) => panic!("Couldn't backup file {}: {}", display, why),
        Ok(file) => file,
    };
    println!("Backup done {:?} -> {:?}", &path, &path_backup);
    Ok(())
}

fn get_line_type(line: &String, n: &usize) -> Result<Line, anyhow::Error> {
    let re_first = Regex::new(r"^(.*?) ").unwrap();
    let matches = re_first.captures(&line);
    let first_thing = match matches {
        Some(m) => m.get(1).unwrap().as_str().to_string(),
        None => String::from("")
    };
    let re_date = Regex::new(r"^(\d{4}-[01]\d-[0-3]\d)").unwrap();
    let re_option = Regex::new(r"^(option)").unwrap();
    let re_comment = Regex::new(r"^(;+)").unwrap();
    let re_indented = Regex::new(r"(?m)(^ +)\S").unwrap();
    let re_empty = Regex::new(r"^.{0}$").unwrap();
    if re_date.is_match(&line) {
        Ok(Line::Date(NaiveDate::parse_from_str(&first_thing, "%Y-%m-%d").unwrap()))
    } else if re_option.is_match(&line) {
        Ok(Line::Option)
    } else if re_comment.is_match(&line) {
        Ok(Line::Comment)
    } else if re_indented.is_match(&line) {
        Ok(Line::Indent)
    } else if re_empty.is_match(&line) {
        Ok(Line::Empty)
    } else {
        Err(anyhow!("Can't define line {}: \"{}\"", n, line))
    }
}

fn construct_dated_entry(line: &String, date: NaiveDate) -> Result<Entry, anyhow::Error> {
    let re = Regex::new(r"^\d{4}-[01]\d-[0-3]\d (\w+|\*|!)").unwrap();
    let matches = re.captures(&line);
    let directive_string = match matches {
        Some(m) => m.get(1).unwrap().as_str(),
        None => return Err(anyhow!("Couldn't finde entry type."))
    };
    let entry = match directive_string {
        "*" | "!" => Entry::MultiLine(MultiLineEntry{content: line.to_owned(), date: date, entry_type: EntryType::Transaction()}),
        "commodity" => Entry::MultiLine(MultiLineEntry{content: line.to_owned(), date: date, entry_type: EntryType::Commodity()}),
        "price" => Entry::SingleLine(SingleLineEntry{content: line.to_owned(), date: date, entry_type: EntryType::Price()}),
        "open" => Entry::SingleLine(SingleLineEntry{content: line.to_owned(), date: date, entry_type: EntryType::Account()}),
        _ => Entry::SingleLine(SingleLineEntry{content: line.to_owned(), date: date, entry_type: EntryType::OtherEntry()})
            // TODO check if single line is always accurat
    };
    Ok(entry)
}

fn find_entries(mut ledger_file: LedgerFile, n_skip: usize) -> Result<LedgerFile, anyhow::Error> {
    let reader = BufReader::new(&ledger_file.file);
    let mut lines = reader.lines().into_iter();
    let mut line_vec: Vec<(String, Line)> = Vec::new();
    let mut was_comment = false;
    for _i in 0..n_skip {
        let line: String = lines.next().unwrap().unwrap();
        let entry = Entry::SingleLine(SingleLineEntry{content: line,
                                                      date: NaiveDate::from_ymd(1990, 01, 01),
                                                      entry_type: EntryType::Header()});
        ledger_file.entries.push(entry)
    }

    for (mut nn, line) in lines.enumerate() {
        nn += 1;
        let n = nn + n_skip;
        let line: String = line.unwrap();
        let line_type: Line = get_line_type(&line, &n).unwrap();
        line_vec.push((line.clone(), line_type.clone()));
        let entry: Entry = match line_type {
            // Check start of each line
            //  - date
            //      - Check type
            //      - If multiline, keep reading
            //  - option
            //      - save
            //  - comment
            //      - Ignore
            //  - indented
            //      - add to previous transaction
            //  - empty
            //      - Ignore
            Line::Date(d) => construct_dated_entry(&line, d).unwrap(),
            Line::Option => Entry::SingleLine(SingleLineEntry{content: line.to_owned(),
                                                              date: NaiveDate::from_ymd(1990, 01, 01),
                                                               entry_type: EntryType::Option()}),
            Line::Comment => {was_comment = true;
                              Entry::SingleLine(SingleLineEntry{content: line.to_owned(),
                                                               date: NaiveDate::from_ymd(1990, 01, 01),
                                                                entry_type: EntryType::Comment()})},
            Line::Indent => Entry::MultiLine(MultiLineEntry{content: line.to_owned(),
                                                            date: NaiveDate::from_ymd(1990, 01, 01),
                                                            entry_type: EntryType::Indented()}),
            Line::Empty => continue,
        };
        match entry {
            Entry::MultiLine(ref l) => {
                // check if entry is an indented line
                if let EntryType::Indented() = l.entry_type {
                    let last_entry = ledger_file.entries.pop().unwrap();
                    // continue only if last line was a MultiLine-Entry
                    if let Entry::MultiLine(old_entry) = last_entry {
                        let content_new = old_entry.content.to_owned() + "\n" + &l.content;
                        let date_new = old_entry.date;
                        let entry_type_new = old_entry.entry_type;
                        let new_entry = Entry::MultiLine(MultiLineEntry{
                            content: content_new,
                            date: date_new,
                            entry_type: entry_type_new,
                        });
                        ledger_file.entries.push(new_entry);
                    } else {
                        // otherwise panic
                        return Err(anyhow!("Misplaced indented line: Line {}\n\"{:?}\"\n{:?}\n", n, l.content, last_entry))
                    };
                } else {
                    ledger_file.entries.push(entry.clone())
                }
            },
            Entry::SingleLine(ref _l) => ledger_file.entries.push(entry.clone()),
        };
    };
    Ok(ledger_file)
}


fn get_section_variant(entry: &str) -> Result<EntryType, anyhow::Error> {
//["Accounts", "Options", "Commodities", "Other Entries", "Prices", "Transactions"]
    let entry_type = match entry {
        "Accounts" => EntryType::Account(),
        "Options" => EntryType::Option(),
        "Commodities" => EntryType::Commodity(),
        "Other Entries" => EntryType::OtherEntry(),
        "Prices" => EntryType::Price(),
        "Transactions" => EntryType::Transaction(),
        "Header" => EntryType::Header(),
        _ => return Err(anyhow!("Not handled Section Type \"{}\"", entry))
    };
    Ok(entry_type)
}

fn get_entry_type(entry: &Entry) -> Result<&EntryType, anyhow::Error> {
    let entry_type = match entry {
        Entry::MultiLine(mle) => &mle.entry_type,
        Entry::SingleLine(sle) => &sle.entry_type,
    };
    Ok(entry_type)
}

fn sort_entries(entries: Vec<Entry>) -> Result<Vec<Entry>, anyhow::Error> {
    let mut sorted_entries: Vec<Entry> = Vec::new();
    let deco = ";;;;".to_string();
    for section in SECTIONS {
        if section != "Header" {
            let section_string = {deco.clone() + &";".repeat(section.len()) + &deco + "\n" +
                                &deco + section + &deco + "\n" +
                                &deco + &";".repeat(section.len()) + &deco};
            let section_entry = Entry::MultiLine(MultiLineEntry{content: section_string,
                                                                date: NaiveDate::from_ymd(1990, 01, 01),
                                                                entry_type: EntryType::Section()});
            sorted_entries.push(section_entry);
        }
        let section_variant = get_section_variant(section).unwrap();
        let entries_iter = entries.iter();
        entries_iter.filter(|e| mem::discriminant(get_entry_type(&e).unwrap()) == mem::discriminant(&section_variant))
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
    println!("Selected beancount file is {:?}", &args.path);
    backup_file(&args.path)?;
    let mut ledger_file = read_file(&args.path).unwrap();
    ledger_file = find_entries(ledger_file, args.skipn).unwrap();
    ledger_file.entries = sort_entries(ledger_file.entries).unwrap();
    ledger_file.write_ledger_file(&args.out).unwrap();
    Ok(())
}


#[cfg(test)]
mod test {
    use super::*;
}
