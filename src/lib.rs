extern crate hyper;
extern crate select;
extern crate regex;
extern crate cookie;
extern crate time;
extern crate multipart;
#[macro_use] extern crate mdo;

use std::fmt::Display;
use std::str::FromStr;

use regex::Regex;

use std::collections::HashMap;

use hyper::client::Client;
use hyper::client::request::Request;
use hyper::header::{SetCookie,Cookie};
use hyper::status::StatusCode;

use multipart::client::Multipart;

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

impl From<StatusCode> for TabunError {
    fn from(x: StatusCode) -> Self {
        TabunError::NumError(x)
    }
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

const HOST_URL: &'static str = "http://ls.andreymal.org";

impl<'a> TClient<'a> {

    ///Входит на табунчик и сохраняет LIVESTREET_SECURITY_KEY,
    ///если логин или пароль == "" - анонимус.
    ///
    ///# Examples
    ///```no_run
    ///let mut user = libtabun::TClient::new("логин","пароль");
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
                cookies:            CookieJar::new(time::now().to_timespec().sec.to_string().as_bytes()),
            });
        }

        let mut user = TClient::new("","").unwrap();

        let err_regex = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap();
        let hacking_regex = Regex::new("Hacking").unwrap();

        let ls_key_regex = Regex::new(r"LIVESTREET_SECURITY_KEY = '(.+)'").unwrap();

        let page = try!(user.get(&"/login".to_owned()));

        let page_html = page.find(Name("html")).first().unwrap().html();

        user.security_ls_key = ls_key_regex.captures(&page_html).unwrap().at(1).unwrap().to_owned();

        let added_url = "/login/ajax-login?login=".to_owned() + login +
            "&password=" + pass + "&security_ls_key=" + user.security_ls_key.as_str();

        let res = try!(user.get(&added_url));

        let res = res.nth(0).unwrap().text();
        let res = res.as_str();


        if hacking_regex.is_match(res) {
            Err(TabunError::HackingAttempt)
        } else if err_regex.is_match(res) {
            let err = err_regex.captures(res).unwrap();
            Err(TabunError::Error(err.at(1).unwrap().to_owned(),err.at(2).unwrap().to_owned())) 
        } else {
            let page = try!(user.get(&"".to_owned()));

            user.name = page.find(Class("username")).first().unwrap().text();

            Ok(user)
        }
    }
    
    fn get(&mut self,url: &String) -> Result<Document,StatusCode>{
        let full_url = HOST_URL.to_owned() + &url;

        let mut res = self.client.get(
            &full_url)
            .header(Cookie::from_cookie_jar(&self.cookies))
            .send()
            .unwrap();

        if res.status != hyper::Ok { return Err(res.status) }

        let mut buf = String::new();
        res.read_to_string(&mut buf).unwrap();

        let cookie = if res.headers.has::<SetCookie>() {  
            Some(res.headers.get::<SetCookie>().unwrap())
        } else {
            None
        };

        if let Some(_) = cookie {
            cookie.unwrap().apply_to_cookie_jar(&mut self.cookies);
        }

        Ok(Document::from(&*buf))
    }

    ///Оставить коммент к какому-нибудь посту, reply=0 - ответ на сам пост,
    ///иначе на чей-то коммент
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.comment(1234,"Привет!",0);
    ///```
    ///
    ///# Errors
    ///Может возвращать `TabunError::NumError`, если
    ///поста не существует
    pub fn comment(&mut self,post_id: i32, body : &str, reply: i32) -> Result<i64,TabunError>{
        use mdo::option::{bind};

        let id_regex = Regex::new("\"sCommentId\":(\\d+)").unwrap();
        let err_regex = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap();

        let url = "/blog/ajaxaddcomment?security_ls_key=".to_owned() + self.security_ls_key.as_str() +
            "&cmt_target_id=" + post_id.to_string().as_str() + "&reply=" + reply.to_string().as_str() +
            "&comment_text=" + body;

        let res = try!(self.get(&url));

        let res = res.nth(0).unwrap().text();
        let res = res.as_str();

        if err_regex.is_match(res) {
            let err = err_regex.captures(res).unwrap();
            return Err(TabunError::Error(err.at(1).unwrap().to_owned(),err.at(2).unwrap().to_owned()));
        }

        Ok(mdo!(
            captures    =<< id_regex.captures(res);
            r           =<< captures.at(1);
            ret r.parse::<i64>().ok()
        ).unwrap())
    }

    ///Получить комменты из некоторого поста
    ///в виде HashMap ID-Коммент
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_comments("lighthouse",157807);
    ///```
    ///
    ///# Errors
    ///Может возвращать `TabunError::NumError`, если
    ///поста не существует
    pub fn get_comments(&mut self,blog: &str, post_id: i32) -> Result<HashMap<i64,Comment>,TabunError> {
        let mut ret = HashMap::new();

        let ref url = "/blog/".to_owned() + blog + "/".to_owned().as_str() + post_id.to_string().as_str() + ".html".to_string().as_str();
        let page = try!(self.get(url));

        let comments = page.find(And(Name("div"),Class("comments")));
        for wrapper in comments.find(And(Name("div"),Class("comment-wrapper"))).iter() {
            let mut parent = 0;
            if wrapper.parent().unwrap().is(And(Name("div"),Class("comment-wrapper"))) {
                parent = wrapper.attr("id").unwrap().split("_").collect::<Vec<_>>()[3].parse::<i32>().unwrap();
            }

            for comm in wrapper.find(Name("section")).iter() {
                let text = comm.find(And(Name("div"),Class("text"))).first().unwrap().inner_html().clone();
                let text = text.as_str();

                let id = comm.attr("id").unwrap().split("_").collect::<Vec<_>>()[2].parse::<i64>().unwrap();

                let author = comm.find(And(Name("li"),Class("comment-author")))
                    .find(Name("a"))
                    .first()
                    .unwrap();
                let author = author.attr("href").unwrap().split("/").collect::<Vec<_>>()[4];

                let date = comm.find(Name("time")).first().unwrap();
                let date = date.attr("datetime").unwrap();

                let votes = comm.find(And(Name("span"),Class("vote-count")))
                    .first()
                    .unwrap()
                    .text().parse::<i32>().unwrap();
                ret.insert(id,Comment{
                    body:   text.to_owned(),
                    id:     id,
                    author: author.to_owned(),
                    date:   date.to_owned(),
                    votes:  votes,
                    parent: parent,
                });
            }
        }
        return Ok(ret);
    }

    ///Получает ID блога по его имени
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///let blog_id = user.get_blog_id("lighthouse").unwrap();
    ///assert_eq!(blog_id,15558);
    ///```
    ///
    ///# Errors
    ///Возвращает `TabunError::NumError`, если блога не существует
    pub fn get_blog_id(&mut self,name: &str) -> Result<i32,TabunError> {
        use mdo::option::{bind,ret};

        let url = "/blog/".to_owned() + name;
        let page = try!(self.get(&url));

        Ok(mdo!(
            x =<< page.find(And(Name("div"),Class("vote-item"))).first();
            x =<< x.find(Name("span")).first();
            x =<< x.attr("id");
            x =<< x.split("_").collect::<Vec<_>>().last();
            x =<< x.parse::<i32>().ok();
            ret ret(x)
        ).unwrap())
    }

    #[allow(unused_must_use)]
    pub fn add_post(&mut self, blog_id: i32, title: &str, body: &str, tags: &str) -> Result<i32,TabunError> {
        use mdo::option::{bind};

        let url = HOST_URL.to_owned() + "/topic/add";
        let mut request = Request::new(hyper::method::Method::Post,
                               hyper::Url::from_str(&url).unwrap()).unwrap();
        request.headers_mut().set(Cookie::from_cookie_jar(&self.cookies));

        let mut req = Multipart::from_request(request).unwrap();


        req.write_text("topic_type","topic");
        req.write_text("security_ls_key",self.security_ls_key.clone());
        req.write_text("blog_id",blog_id.to_string());
        req.write_text("topic_title",title);
        req.write_text("topic_text",body);
        req.write_text("topic_tags",tags);
        req.write_text("submit_topic_publish","Опубликовать");

        let res = req.send().unwrap();

        if res.status != hyper::status::StatusCode::MovedPermanently { return Err(TabunError::NumError(res.status)) }

        let cookie = if res.headers.has::<SetCookie>() {
            Some(res.headers.get::<SetCookie>().unwrap())
        } else {
            None
        };

        if let Some(_) = cookie {
            cookie.unwrap().apply_to_cookie_jar(&mut self.cookies);
        }

        let r = std::str::from_utf8(&res.headers.get_raw("location").unwrap()[0]).unwrap();

        Ok(mdo!(
            regex       =<< Regex::new(r"(\d+).html$").ok();
            captures    =<< regex.captures(r);
            r           =<< captures.at(1);
            ret r.parse::<i32>().ok()
        ).unwrap())
    }
}

