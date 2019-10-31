use headless_chrome::Browser;
use std::time::Instant;

#[derive(Debug)]
pub struct AfterMarketPriceData {
    symbol: String,
    percentage: f64,
    date: Instant,
}

fn scrape_cnn_after_market_datasource() -> Result<Vec<AfterMarketPriceData>, failure::Error> {
    let mut after_market_data = Vec::new();

    let browser = Browser::default()?;

    let tab = browser.wait_for_initial_tab()?;

    // navigate to the afterhours info webpage on CNN
    tab.navigate_to("https://money.cnn.com/data/afterhours/")?;

    // locate the HTML table with the afterhours trading Gainers and Losers
    tab.wait_for_element("div#wsod_marketMoversContainer")?;

    let table_price_changes = tab.find_element("div#wsod_marketMoversContainer")?;

    let node = table_price_changes.get_description()?;
    let table = node.find(|n| n.node_name == "TBODY").unwrap();

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
        let first_column = row.find(|n| {
            let attrs = n.attributes.clone(); // TODO learn why compiler won't let me because
            // n.attributes so I don't have to use the slower `.clone`
            attrs.unwrap_or_default().get("class") == Some(&"wsod_firstCol".to_string())
        }).unwrap();

        let ticker_symbol: String = first_column
            .find(|n| n.node_name == "#text").unwrap().node_value.to_string();

        // first, make sure this is a positive price change. We do not want
        // to scrape tickers with negative price changes (cuz we don't think those will make money!)
        let neg_price_column = row.find(|n| {
            let attrs = n.attributes.clone(); // TODO learn why compiler won't let me because
            // n.attributes so I don't have to use the slower `.clone`
            attrs.unwrap_or_default().get("class") == Some(&"negChangePct".to_string())
        });

        if neg_price_column.is_some() {
            break; // ignore this row
        }

        let third_column = row.find(|n| {
            let attrs = n.attributes.clone(); // TODO learn why compiler won't let me because
            // n.attributes so I don't have to use the slower `.clone`
            attrs.unwrap_or_default().get("class") == Some(&"posChangePct".to_string())
        }).unwrap();

        // this gives us a String of the form "+7.06%"
        let mut price_perc_change: String = third_column
            .find(|n| n.node_name == "#text").unwrap().node_value.to_string();

        // strip away the +/- and % chars
        price_perc_change.remove(price_perc_change.len() - 1);
        price_perc_change.remove(0);

        let price_perc_change: f64 = price_perc_change.parse()?;

        let now = Instant::now();

        let price_data = AfterMarketPriceData {
            symbol: ticker_symbol,
            percentage: price_perc_change,
            date: now
        };

        after_market_data.push(price_data);
    }

    // we also want the S&P price change, because our strategy takes the movement
    // of the S&P 500 into account (if it's largely positive, then we believe the
    // market will have greater liklihood to buy the trending aftermarket trades)

    let standard_poors_price_change = tab.find_element("div#premkContent1")?;
    let node = standard_poors_price_change.get_description()?;

    let sp_row = node.find(|n| {
        let attrs = n.attributes.clone(); // TODO learn why compiler won't let me because
        // n.attributes so I don't have to use the slower `.clone`
        attrs.unwrap_or_default().get("class") == Some(&"wsod_futureQuote wsod_futureQuoteFirst".to_string())
    }).unwrap();

    let sp_price_changes = sp_row.find(|n| {
        let attrs = n.attributes.clone(); // TODO learn why compiler won't let me because
        // n.attributes so I don't have to use the slower `.clone`
        attrs.unwrap_or_default().get("class") == Some(&"wsod_bold wsod_aRight".to_string())
    }).unwrap();

    // this will get us a String of the form "-0.71%"
    let mut sp_perc_change = sp_price_changes.find(|n| {
        n.node_value.contains("%")
    }).unwrap().node_value.clone();

    // strip away the % char
    sp_perc_change.remove(sp_perc_change.len() - 1);

    let sp_perc_change: f64 = sp_perc_change.parse()?;

    let now = Instant::now();

    let price_data = AfterMarketPriceData {
        symbol: "S&P".to_string(),
        percentage: sp_perc_change,
        date: now
    };

    after_market_data.push(price_data);

    Ok(after_market_data)
}


fn main() {
    println!("{:?}", scrape_cnn_after_market_datasource().unwrap());
}
