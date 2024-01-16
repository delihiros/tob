use std::{borrow::Cow, env};

use regex::Regex;
use reqwest::blocking::Client;
use scraper::ElementRef;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Company {
    name: Cow<'static, str>,
    url: Cow<'static, str>,
    children: Vec<Company>,
    board: Option<Board>,
}

impl Company {
    fn new(name: impl Into<Cow<'static, str>>, url: impl Into<Cow<'static, str>>) -> Self {
        return Self {
            name: name.into(),
            url: url.into(),
            children: [].to_vec(),
            board: None,
        };
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Board {
    company_name: Cow<'static, str>,
    boards: Vec<Node>,
    members: Vec<Node>,
    company_tree: Vec<Node>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Node {
    name: Cow<'static, str>,
    title: Cow<'static, str>,
    children: Vec<Node>,
}

fn get_board(url: &str) -> Option<Board> {
    let client = Client::new();
    let response = client.get(url).send();
    let html_content = response.unwrap().text().unwrap();
    let document = scraper::Html::parse_document(&html_content);
    let board_selector = scraper::Selector::parse("div.board").unwrap();
    let board = document.select(&board_selector).next().unwrap();
    let obtree_selector = scraper::Selector::parse("ul.obTree").unwrap();
    let obtree = document.select(&obtree_selector).next().unwrap();
    parse_board(board)
}

fn parse_obtree(obtree: ElementRef<'_>) -> Option<Company> {
    None
}

fn parse_board(board: ElementRef<'_>) -> Option<Board> {
    let board_selector =
        scraper::Selector::parse("div.board-column > ul.board-block > li > div").unwrap();

    let board_branch_row_selector =
        scraper::Selector::parse("div.board-branch > div.board-branch-row").unwrap();

    let contacts = board.select(&board_selector);
    let title_selector = scraper::Selector::parse("div.oc-title").unwrap();
    let name_selector = scraper::Selector::parse("div.oc-name").unwrap();
    let boarding_members = contacts
        .map(|contact| {
            let name = contact
                .select(&name_selector)
                .next()
                .unwrap()
                .text()
                .next()
                .unwrap();
            let title = contact
                .select(&title_selector)
                .next()
                .unwrap()
                .text()
                .next()
                .unwrap();
            Node {
                name: name.trim().to_string().into(),
                title: title.trim().to_string().into(),
                children: [].to_vec(),
            }
        })
        .collect::<Vec<_>>();

    let mut members: Vec<Node> = vec![];

    let branch_rows = board.select(&board_branch_row_selector);

    let parent_contact_selector =
        scraper::Selector::parse("div.ocN1 > ul.board-block > li > div").unwrap();
    let children_contact_selector =
        scraper::Selector::parse("div.ocN2 > div > ul.board-block > li > div").unwrap();

    for branch_row in branch_rows {
        let parent_contact = branch_row.select(&parent_contact_selector).next().unwrap();
        let name = parent_contact.select(&name_selector).next().unwrap();
        let title = parent_contact.select(&title_selector).next().unwrap();
        let mut grand_children: Vec<Node> = vec![];
        for grand_child in branch_row.select(&children_contact_selector) {
            let name = grand_child.select(&name_selector).next().unwrap();
            let title = grand_child.select(&title_selector).next().unwrap();
            grand_children.push(Node {
                name: name.text().next().unwrap().trim().to_string().into(),
                title: title.text().next().unwrap().trim().to_string().into(),
                children: [].to_vec(),
            })
        }
        let member = Node {
            name: name.text().next().unwrap().trim().to_string().into(),
            title: title.text().next().unwrap().trim().to_string().into(),
            children: grand_children,
        };
        members.push(member)
    }

    let board = Board {
        company_name: "".into(),
        boards: boarding_members,
        members: members,
        company_tree: [].to_vec(),
    };

    Some(board)
}

fn search_companies(name: &str) -> Vec<Company> {
    const BASE_URL: &str = "https://www.theofficialboard.jp";
    let client = Client::new();
    let query = vec![("q", name)];
    let response = client
        .get(BASE_URL.to_string() + "/company/search")
        .query(&query)
        .send();
    let html_content = response.unwrap().text().unwrap();
    let document = scraper::Html::parse_document(&html_content);
    let company_title_selector =
        scraper::Selector::parse("#results > ul > li > div > div > div.companyTitle").unwrap();
    let results_list = document.select(&company_title_selector);

    let re = Regex::new(r"'(?<url>[^']*)'").unwrap();

    let mut companies: Vec<Company> = Vec::new();
    for result in results_list {
        let url = result.value().attr("onclick");
        let name = result
            .select(&scraper::Selector::parse("span.nom_entr").unwrap())
            .next()
            .unwrap()
            .text()
            .next()
            .unwrap()
            .to_string();
        if let Some(value) = url {
            if let Some(caps) = re.captures(value) {
                companies.push(Company::new(name, BASE_URL.to_string() + &caps["url"]))
            }
        }
    }
    return companies;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let companies = search_companies(&args[1]);
    let first_one = &companies[0];
    let mut board = get_board(&first_one.url).unwrap();
    board.company_name = first_one.name.clone();
    println!("{}", json!(board))
}
