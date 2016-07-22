extern crate hyper;
extern crate select;
extern crate regex;
extern crate cookie;
extern crate time;

use std::fmt::Display;

use regex::Regex;

use hyper::Client;
use hyper::header::{SetCookie,Cookie};
use hyper::status::StatusCode;

use std::io::Read;

use select::document::Document;
use select::predicate::{Class, Name, And};

use cookie::CookieJar;

#[derive(Debug)]
pub enum TabunError {
    ///На случай `Hacking attempt!`
    HackingAttempt,
    ///Ошибка с названием и описанием,
    ///обычно соответствует табуновским
    ///всплывающим сообщениям
    ///TODO: сделать их читаемыми
    Error(String,String),
    ///Ошибка с номером, вроде 404 и 403
    NumError(StatusCode)
}

///Клиент табуна
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

    ///Входит на табунчик и сохраняет LIVESTREET_SECURITY_KEY,
    ///если логин или пароль == "" - анонимус.
    ///
    ///# Examples
    ///```
    ///let mut user = TClient.new("логин","пароль")
    ///```
    ///
    ///# Errors
    ///Если войти не удалось, то возвращает `TabunError::Error`
    ///с описание ошибки
    pub fn new(login: &str, pass: &str) -> Result<TClient<'a>,TabunError> {
        if login == "" || pass == "" {
            return Ok(TClient{
                name:               "".to_owned(),
                security_ls_key:    "".to_owned(),
                client:             Client::new(),
                cookies:            CookieJar::new(&*time::now().to_timespec().sec.to_string().as_bytes()),
            });
        }

        let mut user = TClient::new("","").unwrap();

        let err_regex = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap();
        let hacking_regex = Regex::new("Hacking").unwrap();

        let ls_key_regex = Regex::new(r"LIVESTREET_SECURITY_KEY = '(.+)'").unwrap();

        let page = match user.get(&"/login".to_owned()) {
            Ok(x) => x,
            Err(x) => return Err(TabunError::NumError(x))
        };

        let page_html = page.find(Name("html")).first().unwrap().html();

        user.security_ls_key = ls_key_regex.captures(&*page_html).unwrap().at(1).unwrap().to_owned();

        let added_url = "/login/ajax-login?login=".to_owned() + login +
            "&password=" + pass + "&security_ls_key=" + user.security_ls_key.as_str();

        let res = match user.get(&added_url) {
            Ok(x) => x.nth(0).unwrap().text(),
            Err(x) => return Err(TabunError::NumError(x))
        };

        let res = res.as_str();


        if hacking_regex.is_match(res) {
            Err(TabunError::HackingAttempt)
        } else if err_regex.is_match(res) {
            let err = err_regex.captures(res).unwrap();
            Err(TabunError::Error(err.at(1).unwrap().to_owned(),err.at(2).unwrap().to_owned())) 
        } else {
            let page = match user.get(&"".to_owned()) {
                Ok(x) => x,
                Err(x) => return Err(TabunError::NumError(x))
            };

            user.name = page.find(Class("username")).first().unwrap().text();

            Ok(user)
        }
    }
    
    fn get(&mut self,url: &String) -> Result<Document,StatusCode>{
        let full_url = "https://tabun.everypony.ru".to_owned() + &url;

        let mut res = self.client.get(
            &full_url)
            .header(Cookie::from_cookie_jar(&self.cookies))
            .send()
            .unwrap();

        if res.status != hyper::Ok { return Err(res.status) }

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

        Ok(Document::from(&*buf))
    }

    ///Оставить коммент к какому-нибудь посту, reply=0 - ответ на сам пост,
    ///иначе на чей-то коммент
    ///
    ///# Examples
    ///```rust,no_run
    ///# let mut user = TClient.new("логин","пароль");
    ///user.comment(1234,"Привет!",0)
    ///```
    ///
    ///# Errors
    ///Может возвращать `TabunError::NumError`, если
    ///поста не существует
    pub fn comment(&mut self,post_id: i32, body : &str, reply: i32) -> Result<i64,TabunError>{
        let id_regex = Regex::new("\"sCommentId\":(\\d+)").unwrap();
        let err_regex = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap();

        let url = "/blog/ajaxaddcomment?security_ls_key=".to_owned() + self.security_ls_key.as_str() +
            "&cmt_target_id=" + post_id.to_string().as_str() + "&reply=" + reply.to_string().as_str() +
            "&comment_text=" + body;

        let res = match self.get(&url) {
            Ok(x) => x.nth(0).unwrap().text(),
            Err(x) => return Err(TabunError::NumError(x))
        };

        let res = res.as_str();

        if err_regex.is_match(res) {
            let err = err_regex.captures(res).unwrap();
            return Err(TabunError::Error(err.at(1).unwrap().to_owned(),err.at(2).unwrap().to_owned()));
        }

        Ok(id_regex.captures(res)
            .unwrap()
            .at(1)
            .unwrap()
            .parse::<i64>().unwrap())
    }

    ///Получить комменты из некоторого поста
    ///
    ///# Examples
    ///```
    ///# let mut user = TClient.new("логин","пароль");
    ///user.get_comments("lighthouse",157807)
    ///```
    ///
    ///# Errors
    ///Может возвращать `TabunError::NumError`, если
    ///поста не существует
    pub fn get_comments(&mut self,blog: &str, post_id: i32) -> Result<Vec<Comment>,TabunError> {
        let mut ret = Vec::with_capacity(0);

        let url = "/blog/".to_owned() + blog + "/".to_owned().as_str() + post_id.to_string().as_str() + ".html".to_string().as_str();
        let page = match self.get(&url) {
            Ok(x) => x,
            Err(x)  => return Err(TabunError::NumError(x)),
        };

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
        return Ok(ret);
    }

    ///Получает ID блога по его имени
    ///
    ///# Examples
    ///```
    ///# let mut user = TClient.new("логин","пароль");
    ///let blog_id = user.get_blog_id("lighthouse").unwrap();
    ///assert_eq!(blog_id,15558)
    ///```
    ///
    ///# Errors
    ///Возвращает `TabunError::NumError` если блога не существует
    pub fn get_blog_id(&mut self,name: &str) -> Result<i32,TabunError> {
        let url = "/blog/".to_owned() + name;
        let page = match self.get(&url) {
            Ok(x) => x,
            Err(x) => return Err(TabunError::NumError(x))
        };

        Ok(page.find(And(Name("div"),Class("vote-item")))
            .first().unwrap()
            .find(Name("span"))
            .first().unwrap()
            .attr("id").unwrap()
            .split("_").collect::<Vec<&str>>()
            .last().unwrap()
            .parse::<i32>().unwrap())
    }
}

