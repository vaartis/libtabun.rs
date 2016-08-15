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
use select::predicate::{Class, Name};

use cookie::CookieJar;

macro_rules! map(
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     };
);

mod add;
mod get;

//Перечисления

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

pub enum CommentType {
    ///Комментарий к посту
    Post,

    ///Ответ на личное сообщение
    Talk
}

//Структуры

///Клиент табуна
pub struct TClient<'a> {
    pub name:               String,
    pub security_ls_key:    String,
    client:                 Client,
    cookies:                CookieJar<'a>,
}

#[derive(Debug,Clone)]
pub struct Comment {
    pub body:   String,
    pub id:     i64,
    pub author: String,
    pub date:   String,
    pub votes:  i32,
    pub parent: i64,
}

#[derive(Debug,Clone)]
pub struct Post {
    pub title:          String,
    pub body:           String,
    pub date:           String,
    pub tags:           Vec<String>,
    pub comments_count: i32,
    pub author:         String,
    pub id:             i32,
}

#[derive(Debug,Clone)]
pub struct EditablePost {
    pub title:          String,
    pub body:           String,
    pub tags:           Vec<String>,
}

#[derive(Debug,Clone)]
pub struct InBlogs {
    ///Созданные пользователем блоги
    created: Vec<String>,

    ///Блоги, в которых пользователь является администратором
    admin: Vec<String>,

    ///Блоги, в которых пользователь является модератором
    moderator: Vec<String>,

    ///Блоги, в которых пользователь состоит
    member: Vec<String>
}

#[derive(Debug,Clone)]
pub struct UserInfo {
    pub username:       String,
    pub realname:       String,

    ///Силушка
    pub skill:          f32,
    pub id:             i32,

    ///Кармочка
    pub rating:         f32,
 
    ///URL картинки, иногда с `//`, иногда с `https://`
    pub userpic:        String,
    pub description:    String,

    ///Информация вроде даты рождения и последнего визита,
    ///поля называются как на сайте
    pub other_info:     HashMap<String,String>,

    ///Блоги, которые юзер создал/состоит в них/модерирует
    pub blogs:          InBlogs,

    ///Кол-во публикаций
    pub publications:   i32,

    ///Кол-во избранного
    pub favourites:     i32,

    ///Кол-во друзей
    pub friends:        i32
}

///Диалог в личных сообщениях
#[derive(Debug,Clone)]
pub struct Talk {
    pub title:  String,
    pub body:   String,

    ///Участники
    pub users:  Vec<String>,
    pub comments: HashMap<i64, Comment>,
    pub date:   String
}

//Реализации

impl From<StatusCode> for TabunError {
    fn from(x: StatusCode) -> Self {
        TabunError::NumError(x)
    }
}

impl Display for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Comment({},\"{}\",\"{}\")", self.id, self.author, self.body)
    }
}

impl Display for Post {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Post({},\"{}\",\"{}\")", self.id, self.author, self.body)
    }
}

impl Display for UserInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "UserInfo({},\"{}\",\"{}\")", self.username, self.skill, self.rating)
    }
}

///URL сайта. Ибо по идее может работать и с другими штуками ня лайвстрите
pub const HOST_URL: &'static str = "https://tabun.everypony.ru";

impl<'a> TClient<'a> {

    ///Входит на табунчик и сохраняет LIVESTREET_SECURITY_KEY,
    ///если логин или пароль == "" - анонимус.
    ///
    ///# Examples
    ///```no_run
    ///let mut user = libtabun::TClient::new("логин","пароль");
    ///```
    pub fn new(login: &str, pass: &str) -> Result<TClient<'a>,TabunError> {
        if login.is_empty() || pass.is_empty() {
            return Ok(TClient{
                name:               String::new(),
                security_ls_key:    String::new(),
                client:             Client::new(),
                cookies:            CookieJar::new(time::now().to_timespec().sec.to_string().as_bytes()),
            });
        }

        let mut user = TClient::new("","").unwrap();

        let err_regex = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap();

        let ls_key_regex = Regex::new(r"LIVESTREET_SECURITY_KEY = '(.+)'").unwrap();

        let page = try!(user.get(&"/login".to_owned()));

        let page_html = page.find(Name("html")).first().unwrap().html();

        user.security_ls_key = ls_key_regex.captures(&page_html).unwrap().at(1).unwrap().to_owned();

        let added_url = format!("/login/ajax-login?login={}&password={}&security_ls_key={}"
                                , login, pass, user.security_ls_key);

        let res = try!(user.get(&added_url));

        let res = res.nth(0).unwrap().text();
        let res = res.as_str();


        if res.contains("Hacking") {
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

    fn get(&mut self,url: &str) -> Result<Document,StatusCode>{
        let full_url = format!("{}{}", HOST_URL, url);

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

    fn multipart(&mut self,url: &str, bd: HashMap<&str,&str>) -> Result<hyper::client::Response,StatusCode> {
        let url = format!("{}{}", HOST_URL, url);
        let mut request = Request::new(hyper::method::Method::Post,
                               hyper::Url::from_str(&url).unwrap()).unwrap();
        request.headers_mut().set(Cookie::from_cookie_jar(&self.cookies));

        let mut req = Multipart::from_request(request).unwrap();

        for (param,val) in bd {
            let _ = req.write_text(param,val);
        }

        let res = req.send().unwrap();

        if res.status != hyper::Ok && res.status != hyper::status::StatusCode::MovedPermanently { return Err(res.status) }

        Ok(res)
    }

    ///Редактирует пост
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///let blog_id = user.get_blog_id("computers").unwrap();
    ///user.edit_post(157198,blog_id,"Новое название", "Новый текст", vec!["тэг".to_string()],false);
    ///```
    pub fn edit_post(&mut self, post_id: i32, blog_id: i32, title: &str, body: &str, tags: Vec<String>, forbid_comment: bool) -> Result<i32,TabunError> {
        use mdo::option::{bind};

        let blog_id = blog_id.to_string();
        let key = self.security_ls_key.clone();
        let forbid_comment = if forbid_comment { "1" } else { "0" };
        let tags = tags.iter().fold(String::new(), |acc, x| acc + &format!("{},", *x));

        let bd = map![
            "topic_type"            =>  "topic",
            "blog_id"               =>  &blog_id,
            "topic_title"           =>  title,
            "topic_text"            =>  body,
            "topic_tags"            =>  &tags,
            "submit_topic_publish"  =>  "Опубликовать",
            "security_ls_key"       =>  &key,
            "topic_forbid_comment"  =>  &forbid_comment
        ];

        let res = try!(self.multipart(&format!("/topic/edit/{}",post_id), bd));

        let r = std::str::from_utf8(&res.headers.get_raw("location").unwrap()[0]).unwrap();

        Ok(mdo!(
            regex       =<< Regex::new(r"(\d+).html$").ok();
            captures    =<< regex.captures(r);
            r           =<< captures.at(1);
            ret r.parse::<i32>().ok()
        ).unwrap())
    }

    ///Подписаться/отписаться от комментариев к посту.
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.comments_subscribe(157198,false);
    ///```
    pub fn comments_subscribe(&mut self, post_id: i32, subscribed: bool) {
        let subscribed = if subscribed { "1" } else { "0" };

        let post_id = post_id.to_string();
        let key = self.security_ls_key.clone();

        let body = map![
        "target_type"       =>  "topic_new_comment",
        "target_id"         =>  post_id.as_str(),
        "value"             =>  subscribed,
        "mail"              =>  "",
        "security_ls_key"   => &key
        ];

        let _ = self.multipart("/subscribe/ajax-subscribe-toggle",body);
    }

    ///Загружает картинку по URL, попутно вычищая табуновские бэкслэши из ответа
    pub fn upload_image_from_url(&mut self, url: &str) -> Result<String,TabunError>{
        let key = self.security_ls_key.clone();
        let url_regex = Regex::new(r"img src=\\&quot;(.+)\\&quot;").unwrap();
        let mut res_s = String::new();
        let mut res = try!(self.multipart("/ajax/upload/image", map!["title" => "", "img_url" => url, "security_ls_key" => &key]));
        let _ = res.read_to_string(&mut res_s);
        match url_regex.captures(&res_s) {
            Some(x) => Ok(x.at(1).unwrap().to_owned()),
            None    => {
                        let err_regex = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap();
                        let s = res_s.clone();
                        let err = err_regex.captures(&s).unwrap();
                        Err(TabunError::Error(err.at(1).unwrap().to_owned(),err.at(2).unwrap().to_owned()))
            }
        }
    }
}
