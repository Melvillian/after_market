use headless_chrome::Browser;
use std::default::Default;

fn browse_wikipedia() -> Result<(), failure::Error> {
    let browser = Browser::default()?;

    let tab = browser.wait_for_initial_tab()?;

    // navigate to the afterhours info webpage on CNN
    tab.navigate_to("https://money.cnn.com/data/afterhours/")?;

    // locate the HTML table with the afterhours trading Gainers and Losers
    tab.wait_for_element("div#wsod_marketMoversContainer")?;
    let node = tab.find_element("div#wsod_marketMoversContainer")?;
    let description = node.get_description()?;
    let table = description.find(|n| n.node_name == "TBODY").unwrap();

    let rows = table.children.as_ref().unwrap();

    // now that we've located the rows of the Gainers and Losers, we will
    // discard the first row because it is the table header, and then we'll
    // extract the ticker info with positive price changes

    for row in rows.into_iter() {
        let maybe_header = row.find(|n| n.node_value == "Gainers & Losers");
        if maybe_header.is_some() {
            // this is the header of the table, so we skip it because
            // it doesn't contain intersting data
            continue;
        }

        // get the symbol
        println!("{:?}", row.find(|n| n.attributes.unwrap_or_default()
            .get("class") == Some(&"wsod_firstCol".to_string())));
    }

    Ok(())
}


fn main() {
    assert!(browse_wikipedia().is_ok());
    println!("Hello, world!");
}
