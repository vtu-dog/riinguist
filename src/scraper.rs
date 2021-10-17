use std::collections::HashMap;

use regex::Regex;
use scraper::{ElementRef, Html, Selector};

use crate::definition::Definition;

static TERMINOLOGY_URL: &str = "https://riichi.wiki/List_of_terminology_by_alphabetical_order";
static YAKU_URL: &str = "https://riichi.wiki/List_of_yaku";

pub async fn get_definitions() -> HashMap<String, String> {
    build_definition_hashmap(scrape_definitions().await)
}

fn build_definition_hashmap(v: Vec<Definition>) -> HashMap<String, String> {
    v.into_iter()
        .map(|x| {
            (x.name.to_lowercase(), {
                format!(
                    "**{}**{}\n{}",
                    x.name,
                    {
                        match x.details {
                            Some(d) => format!(" ({})", d),
                            None => "".into(),
                        }
                    },
                    x.description
                )
            })
        })
        .collect()
}

async fn scrape_definitions() -> Vec<Definition> {
    let term_doc = scrape_website(TERMINOLOGY_URL).await;
    let yaku_doc = scrape_website(YAKU_URL).await;

    let table_h2_selector = Selector::parse("table, h2").unwrap();
    let table_selector = Selector::parse("table").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    let dd_selector = Selector::parse("dd").unwrap();
    let b_selector = Selector::parse("b").unwrap();
    let i_selector = Selector::parse("i").unwrap();

    let term_list = term_doc
        .select(&table_selector)
        .map(|elem| {
            let mut data = elem.select(&td_selector).take(2);
            let term_and_translation_data = data.next();
            let description_data = data.next();

            let term = term_and_translation_data
                .and_then(|x| x.select(&b_selector).next())
                .map(|x| stringify(x))
                .unwrap();

            let translation = term_and_translation_data
                .and_then(|x| x.select(&i_selector).next())
                .map(|x| stringify(x).to_lowercase());

            let description = description_data.map(|x| stringify(x)).unwrap();

            Definition {
                name: term,
                details: translation,
                description,
            }
        })
        .collect::<Vec<_>>();

    let mut yaku_list = Vec::new();
    let mut hand_value = String::new();

    for elem in yaku_doc.select(&table_h2_selector).skip(1).take(47) {
        if elem.html().contains("<h2") {
            hand_value = stringify(elem).replace("One han closed only", "One han");
            continue;
        }

        let term = elem
            .select(&b_selector)
            .next()
            .map(|x| stringify(x))
            .unwrap();

        let details = elem
            .select(&dd_selector)
            .skip(1)
            .next()
            .map(|x| stringify(x))
            .unwrap()
            .replace(" (", ", ")
            .replace(")", "");

        let description = elem
            .select(&td_selector)
            .skip(1)
            .next()
            .map(|x| stringify(x))
            .unwrap();

        yaku_list.push(Definition {
            name: term,
            details: Some(format!("{}, {}", hand_value, details).to_lowercase()),
            description,
        });
    }

    [term_list, yaku_list].concat()
}

async fn scrape_website(url: &str) -> Html {
    let html = reqwest::get(url)
        .await
        .expect(&format!("Failed to invoke reqwest::get({})", url))
        .text()
        .await
        .expect(&format!("Failed to invoke reqwest::get({}).text()", url));

    Html::parse_document(&html)
}

fn stringify(elem: ElementRef) -> String {
    // yeah, I really don't care about efficiency here, sorry
    let re = Regex::new(r"\s+").unwrap();
    let temp = elem
        .text()
        .filter(|&z| !z.is_empty())
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_owned();

    re.replace_all(&temp, " ").to_string()
}
