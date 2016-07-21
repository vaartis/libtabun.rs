extern crate hyper;
extern crate select;
extern crate regex;
extern crate cookie;
extern crate time;

use std::fmt::Display;

use regex::Regex;

use hyper::Client;
use hyper::header::{SetCookie,Cookie};

use std::io::Read;

use select::document::Document;
use select::predicate::{Class, Name, And};

use cookie::CookieJar;

pub struct TClient<'a> {
    pub name:               String,
    pub security_ls_key:    String,
    client:             Client,
    cookies:            CookieJar<'a>,
}

#[derive(Debug)]
pub struct Comment {
    pub body:   String,
    pub id:     i64,
    pub author: String,
    pub date:   String,
    pub votes:  i32,
    pub parent: i32,
}

impl Display for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Comment({},\"{}\",\"{}\")", self.id, self.author, self.body)
    }
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

    ///Входит на табунчик и сохраняет LIVESTREET_SECURITY_KEY
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

    ///Оставить коммент к какому-нибудь посту, reply=0 - ответ на сам пост,
    ///иначе на чей-то коммент
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

    ///Получить комменты из некоторого поста
    pub fn get_comments(&mut self,blog: &str, post_id: i32) -> Vec<Comment> {
        let mut ret = Vec::with_capacity(0);

        let url = "/blog/".to_owned() + blog + "/".to_owned().as_str() + post_id.to_string().as_str() + ".html".to_string().as_str();
        let page = self.get(&url);
        let comments = page.find(And(Name("div"),Class("comments")));
        for wrapper in comments.find(And(Name("div"),Class("comment-wrapper"))).iter() {
            let mut parent = 0;
            if wrapper.parent().unwrap().is(And(Name("div"),Class("comment-wrapper"))) {
                parent = wrapper.attr("id").unwrap().split("_").collect::<Vec<&str>>()[3].parse::<i32>().unwrap();
            }

            for comm in wrapper.find(Name("section")).iter() {
                let text = comm.find(And(Name("div"),Class("text"))).first().unwrap().inner_html().clone();
                let text = text.as_str();

                let id = comm.attr("id").unwrap().split("_").collect::<Vec<&str>>()[2].parse::<i64>().unwrap();

                let author = comm.find(And(Name("li"),Class("comment-author")))
                    .find(Name("a"))
                    .first()
                    .unwrap();
                let author = author.attr("href").unwrap().split("/").collect::<Vec<&str>>()[4];

                let date = comm.find(Name("time")).first().unwrap();
                let date = date.attr("datetime").unwrap();

                let votes = comm.find(And(Name("span"),Class("vote-count")))
                    .first()
                    .unwrap()
                    .text().parse::<i32>().unwrap();
                ret.push(Comment{
                    body:   text.to_owned(),
                    id:     id,
                    author: author.to_owned(),
                    date:   date.to_owned(),
                    votes:  votes,
                    parent: parent,
                })
            }
        }
        return ret;
    }
}

