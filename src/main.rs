use chrono::{DateTime, Utc};
use core::num::ParseFloatError;
use failure::Fallible;
use headless_chrome::browser::Tab;
use headless_chrome::protocol::dom::Node;
use headless_chrome::Browser;
use std::env;
use std::sync::Arc;

use postgres::{Connection, TlsMode};

extern crate chrono;
extern crate env_logger;
extern crate log;

#[macro_use]
extern crate lazy_static;

/// AfterMarketPriceData holds all the data necessary to track the performance
/// of an after-market-traded stock over time 
#[derive(Debug)]
pub struct AfterMarketPriceData {
    symbol: String,
    percentage: f64,
    date: DateTime<Utc>,
}

const TABLE_NAME: &str = "after_market";

lazy_static! {
    static ref NOW: Option<DateTime<Utc>> = Some(Utc::now());
}

fn main() {
    env_logger::init();
    println!("{:?}", scrape_cnn_after_market_datasource().unwrap());
}

pub fn scrape_cnn_after_market_datasource() -> Result<Vec<AfterMarketPriceData>, failure::Error> {
    let browser = Browser::default()?;
    let tab = initialize_tab(&browser)?;

    // we'll use this to gather all of the ticker data we care about
    let after_market_data = Vec::new();

    let after_market_data = get_after_market_ticker_data(after_market_data, &tab)?;
    let after_market_data = get_standard_and_poors_ticker_data(after_market_data, &tab)?;

    insert_after_market_data_into_db(&after_market_data);

    Ok(after_market_data)
}

fn get_after_market_ticker_data(
    mut v: Vec<AfterMarketPriceData>,
    tab: &Arc<Tab>,
) -> Result<Vec<AfterMarketPriceData>, failure::Error> {
    // locate the HTML table with the afterhours trading Gainers and Losers
    let price_changes_table = tab.wait_for_element("div#wsod_marketMoversContainer")?;

    let node = price_changes_table.get_description()?;
    let table = get_node_with_name(&node, "TBODY");
    let rows = table.children.as_ref().unwrap();

    // now that we've located the rows of the Gainers and Losers, we will
    // discard the first row because it is the table header, and then we'll
    // extract the ticker info with positive price changes
    for row in rows.iter() {
        let maybe_header = row.find(|n| n.node_value == "Gainers & Losers");
        if maybe_header.is_some() {
            // this is the header of the table, so we skip it because
            // it doesn't contain intersting data
            continue;
        }
        // find the column containing the ticker symbol
        let first_column = get_node_with_class(row, "wsod_firstCol");

        let ticker_symbol = get_node_with_name(first_column, "#text")
            .node_value
            .to_string();

        // the data source marks the price change data value with a different
        // HTML class depending on if it's negative or positive so we check for both
        let third_column = match get_node_with_class_as_option(row, "negChangePct") {
            Some(pct) => pct,
            None => get_node_with_class_as_option(row, "posChangePct")
                .unwrap_or_else(|| panic!("couldn't find third_column with row: {:?}", row)),
        };

        // this gives us a String of the form "+7.06%" or "-3.99%"
        let price_perc_change = get_node_with_name(third_column, "#text")
            .node_value
            .to_string();

        let price_perc_change = parse_percentage_str(price_perc_change)?;

        let price_data = AfterMarketPriceData {
            symbol: ticker_symbol,
            percentage: price_perc_change,
            date: NOW.unwrap(),
        };

        v.push(price_data);
    }

    Ok(v)
}

fn get_standard_and_poors_ticker_data(
    mut v: Vec<AfterMarketPriceData>,
    tab: &Arc<Tab>,
) -> Result<Vec<AfterMarketPriceData>, failure::Error> {
    // we also want the S&P price change, because our strategy takes the movement
    // of the S&P 500 into account (if it's largely positive, then we believe the
    // market will have greater liklihood to buy the trending aftermarket trades)
    let standard_poors_price_change = tab.find_element("div#premkContent1")?;
    let node = standard_poors_price_change.get_description()?;

    let sp_row = get_node_with_class(&node, "wsod_futureQuote wsod_futureQuoteFirst");
    let sp_price_changes = get_node_with_class(sp_row, "wsod_bold wsod_aRight");

    // this will get us a String of the form "-0.71%"
    let sp_perc_change = sp_price_changes
        .find(|n| n.node_value.contains("%"))
        .unwrap()
        .node_value
        .clone(); // TODO firgure out how not to be lazy and not clone everything

    let sp_perc_change = parse_percentage_str(sp_perc_change)?;

    let price_data = AfterMarketPriceData {
        symbol: "S&P".to_string(),
        percentage: sp_perc_change,
        date: NOW.unwrap(),
    };
    v.push(price_data);

    Ok(v)
}

/// Given a Node, search through its HTML looking for another Node with a tag
/// whose type is equal to `s`
fn get_node_with_name<'a>(node: &'a Node, s: &str) -> &'a Node {
    node.find(|n| n.node_name == s)
        .unwrap_or_else(|| panic!("couldn't find {:?} tag with node: {:?}", s, node))
}

/// Same as `get_node_with_class_as_option` but unwraps the Option and panics
/// if it is `None`
fn get_node_with_class<'a>(node: &'a Node, s: &str) -> &'a Node {
    match get_node_with_class_as_option(node, s) {
        Some(n) => n,
        _ => panic!("couldn't find {:?}: {:?}", s, node),
    }
}

/// Given a Node, search through its HTML looking for another Node with a tag
/// whose class is equal to 's'
fn get_node_with_class_as_option<'a>(node: &'a Node, s: &str) -> Option<&'a Node> {
    node.find(|n| {
        let attrs = n.attributes.clone(); // TODO learn why compiler won't let me because
                                          // n.attributes so I don't have to use the slower `.clone`
        attrs.unwrap_or_default().get("class") == Some(&s.to_string())
    })
}

/// Strip away the % char so "+7.06%": String becomes 7.06: f64
fn parse_percentage_str(mut price_change: String) -> Result<f64, ParseFloatError> {
    price_change.remove(price_change.len() - 1);
    price_change.parse::<f64>()
}

fn initialize_tab(browser: &Browser) -> Fallible<Arc<Tab>> {
    let tab = browser.wait_for_initial_tab()?;

    // navigate to the after hours info webpage
    let after_market_url = match env::var("AFTER_MARKET_URL") {
        Ok(url) => url,
        Err(error) => panic!("AFTER_MARKET_URL error: {:?}", error),
    };
    tab.navigate_to(&after_market_url)?;

    Ok(tab)
}

fn insert_after_market_data_into_db(after_market_data: &Vec<AfterMarketPriceData>) {
    let conn = Connection::connect(
        env::var("DATABASE_URL").expect("no env var DATABASE_URL"),
        TlsMode::None,
    )
    .unwrap();

    for d in after_market_data.iter() {
        conn.execute(
            &format!(
                "INSERT INTO {} (symbol, percentage, date) VALUES ($1, $2, $3)",
                TABLE_NAME
            ),
            &[&d.symbol, &d.percentage, &d.date],
        )
        .unwrap();
    }
}
