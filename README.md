# Beancount-sort
## Purpose
Sort a beancount file looking like this:
~~~ ledger
2002-01-01 commodity EUR
    name: "Euro"
    asset-class: "cash"
option "operating_currency" "EUR"
2021-01-01 commodity GME
    ; Don't sell!
    name: "Gamestop"
    asset-class: "stock"
2021-01-01 open Assets:Stock
2021-01-01 open Assets:Giro   EUR
2021-01-20 * "Direkthandel" "Aktienkauf"
    Assets:Stock                                   1 GME {69.420 EUR}
    Assets:Giro
2021-01-21 price GME                                420.69 EUR
2021-09-07 * "payee 1" "description 1"
    Expenses:Account1                             15 EUR
    Assets:Giro
2021-09-07 open Expenses:Account1   EUR
2021-09-08 open Assets:Cash   EUR
2021-09-08 open Expenses:Account2   EUR
2021-09-08 * "payee 2" "description 2"
    Expenses:Account2                            3.3 EUR
    Assets:Cash
~~~
to look like this:
~~~ ledger
;;;;;;;;;;;;;;;;
;;;;Accounts;;;;
;;;;;;;;;;;;;;;;
2021-01-01 open Assets:Stock
2021-01-01 open Assets:Giro   EUR
2021-09-07 open Expenses:Account1   EUR
2021-09-08 open Assets:Cash   EUR
2021-09-08 open Expenses:Account2   EUR
;;;;;;;;;;;;;;;
;;;;Options;;;;
;;;;;;;;;;;;;;;
option "operating_currency" "EUR"
;;;;;;;;;;;;;;;;;;;
;;;;Commodities;;;;
;;;;;;;;;;;;;;;;;;;
2002-01-01 commodity EUR
    name: "Euro"
    asset-class: "cash"
2021-01-01 commodity GME
    ; Don't sell!
    name: "Gamestop"
    asset-class: "stock"
;;;;;;;;;;;;;;;;;;;;;
;;;;Other Entries;;;;
;;;;;;;;;;;;;;;;;;;;;
2000-08-01 custom "budget" Assests:Account1       "monthly"         300.00 EUR
;;;;;;;;;;;;;;
;;;;Prices;;;;
;;;;;;;;;;;;;;
2021-01-21 price GME                                420.69 EUR
;;;;;;;;;;;;;;;;;;;;
;;;;Transactions;;;;
;;;;;;;;;;;;;;;;;;;;
2021-01-20 * "Direkthandel" "Aktienkauf"
    Assets:Stock                                   1 GME {420.69 EUR}
    Assets:Giro
2021-09-07 * "payee 1" "description 1"
    Expenses:Account1                             15 EUR
    Assets:Giro
2021-09-08 * "payee 2" "description 2"
    Expenses:Account2                            3.3 EUR
    Assets:Cash
~~~

## Dependencies
1. [beancount](https://beancount.github.io/docs/) (duh)
2. [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
## Installation
Type `cargo install beancount-sort`
## Usage
`beancount-sort --help`
Use with caution! If your output file is the same as the input file the original file will be overwritten!
The program will create a backup of the original file, but if you run the program twice, the first backup will be overwritten.
