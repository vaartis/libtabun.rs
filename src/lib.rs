extern crate hyper;
extern crate select;
extern crate regex;
extern crate cookie;
extern crate time;

use regex::Regex;

use hyper::Client;
use hyper::header::{SetCookie,Cookie};

use std::io::Read;

use select::document::Document;
use select::predicate::{Attr, Class, Name};

use cookie::CookieJar;

pub struct TClient<'a> {
    name:               String,
    security_ls_key:    String,
    client:             Client,
    cookies:            CookieJar<'a>,
}

impl<'a> TClient<'a> {
    pub fn new() -> TClient<'a> {
        TClient{
            name:               "".to_owned(),
            security_ls_key:    "".to_owned(),
            client:             Client::new(),
            cookies:            CookieJar::new(&*time::now().to_timespec().sec.to_string().as_bytes()),
        }
    }
    
    fn get(&mut self,url: &String) -> Document{
        let full_url = "https://tabun.everypony.ru".to_owned() + &url;

        let mut res = self.client.get(
            &full_url)
            .header(Cookie::from_cookie_jar(&self.cookies))
            .send()
            .unwrap();

        assert_eq!(res.status,hyper::Ok);

        let mut buf = String::new();
        res.read_to_string(&mut buf).unwrap();

        let ref cookie = if res.headers.has::<SetCookie>() {  
            Some(res.headers.get::<SetCookie>().unwrap())
        } else {
            None
        };

        match *cookie {
            None => {},
            Some(_) => cookie.unwrap().apply_to_cookie_jar(&mut self.cookies),
        }

        Document::from(&*buf)
    }

    pub fn login(&mut self,login: &str, pass: &str) {
        let ls_key_regex = Regex::new(r"LIVESTREET_SECURITY_KEY = '(.+)'").unwrap();
        let page = self.get(&"/login".to_owned());

        let page_html = page.find(Name("html")).first().unwrap().html();

        self.security_ls_key = ls_key_regex.captures(&*page_html).unwrap().at(1).unwrap().to_owned();

        let added_url = "/login/ajax-login?login=".to_owned() + login +
            "&password=" + pass + "&security_ls_key=" + self.security_ls_key.as_str();
        
        self.get(&added_url);

        let page = self.get(&"".to_owned());

        self.name = page.find(Class("username")).first().unwrap().text();
    }

    pub fn comment(&mut self,post_id: i32, body : &str, reply: i32) -> i32{
        let id_regex = Regex::new("\"sCommentId\":(\\d+)").unwrap();
        let url = "/blog/ajaxaddcomment?security_ls_key=".to_owned() + self.security_ls_key.as_str() +
            "&cmt_target_id=" + post_id.to_string().as_str() + "&reply=" + reply.to_string().as_str() +
            "&comment_text=" + body;

        id_regex.captures(&self.get(&url)
                          .nth(0)
                          .unwrap()
                          .text())
            .unwrap()
            .at(1)
            .unwrap()
            .parse::<i32>().unwrap()
    }
}

