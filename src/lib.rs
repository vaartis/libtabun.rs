/* Main library file
 *
 * Copyright (C) 2016 TyanNN <TyanNN@cocaine.ninja>
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
*/

extern crate hyper;
extern crate select;
extern crate regex;
extern crate cookie;
extern crate multipart;
extern crate unescape;

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
use select::predicate::{Class, Name, And, Attr};

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

///Макро для парса строк и возврата Result,
///парсит st указанным regex, затем вынимает группу номер num
///и парсит в typ
macro_rules! parse_text_to_res(
    { $(regex => $regex:expr, st => $st:expr, num => $n:expr, typ => $typ:ty)+ } => {
        {
            $(
                match Regex::new($regex).ok()
                    .and_then(|x| x.captures($st))
                    .and_then(|x| x.at($n))
                    .and_then(|x| x.parse::<$typ>().ok()) {
                        Some(x) => Ok(x),
                        None    => unreachable!()
                    }
            )+
        }
    };
);

///Макро для удобного unescape
macro_rules! unescape(
    { $($x:expr)+ } => {
        {
            $(
                match unescape::unescape($x) {
                    Some(x) => x,
                    None    => unreachable!()
                }
             )+
        }
    };
);

mod comments;
mod posts;
mod talks;

//Перечисления

#[derive(Debug)]
pub enum TabunError {
    ///На случай `Hacking attempt!`
    HackingAttempt,

    ///Ошибка с названием и описанием,
    ///обычно соответствует табуновским
    ///всплывающим сообщениям
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
    pub body:       String,
    pub id:         u32,
    pub author:     String,
    pub date:       String,
    pub votes:      i32,
    pub parent:     u32,
    pub post_id:    u32,
    pub deleted:    bool
}

#[derive(Debug,Clone)]
pub struct Post {
    pub title:          String,
    pub body:           String,
    pub date:           String,
    pub tags:           Vec<String>,
    pub comments_count: u32,
    pub author:         String,
    pub id:             u32,
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
    pub created: Vec<String>,

    ///Блоги, в которых пользователь является администратором
    pub admin: Vec<String>,

    ///Блоги, в которых пользователь является модератором
    pub moderator: Vec<String>,

    ///Блоги, в которых пользователь состоит
    pub member: Vec<String>
}

#[derive(Debug,Clone)]
pub struct UserInfo {
    pub username:       String,
    pub realname:       String,

    ///Силушка
    pub skill:          f32,
    pub id:             u32,

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
    pub publications:   u32,

    ///Кол-во избранного
    pub favourites:     u32,

    ///Кол-во друзей
    pub friends:        u32
}

///Диалог в личных сообщениях
#[derive(Debug,Clone)]
pub struct Talk {
    pub title:  String,
    pub body:   String,

    ///Участники
    pub users:  Vec<String>,
    pub comments: HashMap<u32, Comment>,
    pub date:   String
}

///Список личных сообщений
#[derive(Debug,Clone)]
pub struct TalkItem {
    pub id: u32,
    pub title:  String,
    pub users:  Vec<String>,
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
                cookies:            CookieJar::new(format!("{:?}",std::time::SystemTime::now()).as_bytes()),
            });
        }

        let mut user = TClient::new("","").unwrap();

        let err_regex = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap();

        let ls_key_regex = Regex::new(r"LIVESTREET_SECURITY_KEY = '(.+)'").unwrap();

        let page = try!(user.get(&"/login".to_owned()))
            .find(Name("html")).first().unwrap().html();

        user.security_ls_key = ls_key_regex.captures(&page).unwrap().at(1).unwrap().to_owned();

        let added_url = format!("/login/ajax-login?login={login}&password={pass}&security_ls_key={key}",
                                login = login,
                                pass = pass,
                                key = user.security_ls_key);

        let res = try!(user.get(&added_url))
            .nth(0).unwrap().text();
        let res = res.as_str();


        if res.contains("Hacking") {
            Err(TabunError::HackingAttempt)
        } else if err_regex.is_match(res) {
            let err = err_regex.captures(res).unwrap();
            Err(TabunError::Error(
                    unescape!(err.at(1).unwrap()),
                    unescape!(err.at(2).unwrap())))
        } else {
            let page = try!(user.get(&"".to_owned()));

            user.name = page.find(Class("username")).first().unwrap().text();

            Ok(user)
        }
    }

    ///Заметка себе: создаёт промежуточный объект запроса, сразу выставляя печеньки,
    ///на случай если надо что-то поменять (как в delete_post)
    fn create_middle_req(&mut self, url: &str) -> hyper::client::RequestBuilder {
        let full_url = format!("{}{}", HOST_URL, url); //TODO: Заменить на concat_idents! когда он стабилизируется
        self.client.get(&full_url)
            .header(Cookie::from_cookie_jar(&self.cookies))
    }

    fn get(&mut self,url: &str) -> Result<Document,StatusCode>{
        let mut res = self.create_middle_req(url)
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
        let url = format!("{}{}", HOST_URL, url); //TODO: Заменить на concat_idents! когда он стабилизируется
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

    ///Загружает картинку по URL, попутно вычищая табуновские бэкслэши из ответа
    pub fn upload_image_from_url(&mut self, url: &str) -> Result<String,TabunError>{
        let key = self.security_ls_key.clone();
        let url_regex = Regex::new(r"img src=\\&quot;(.+)\\&quot;").unwrap();
        let mut res_s = String::new();
        let mut res = try!(self.multipart("/ajax/upload/image", map!["title" => "", "img_url" => url, "security_ls_key" => &key]));
        let _ = res.read_to_string(&mut res_s);
        if let Some(x) = url_regex.captures(&res_s) { Ok(x.at(1).unwrap().to_owned()) } else {
            let err_regex = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap();
            let s = res_s.clone();
            let err = err_regex.captures(&s).unwrap();
            Err(TabunError::Error(
                    unescape!(err.at(1).unwrap()),
                    unescape!(err.at(2).unwrap())))
        }
    }

    ///Получает ID блога по его имени
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///let blog_id = user.get_blog_id("lighthouse").unwrap();
    ///assert_eq!(blog_id,15558);
    ///```
    pub fn get_blog_id(&mut self,name: &str) -> Result<u32,TabunError> {
        let url = format!("/blog/{}", name);
        let page = try!(self.get(&url));

        Ok(page.find(And(Name("div"),Class("vote-item")))
            .find(Name("span")).first()
            .unwrap().attr("id")
            .unwrap().split('_').collect::<Vec<_>>().last()
            .unwrap().parse::<u32>().unwrap())
    }

    ///Получает инфу о пользователе,
    ///если указан как "", то получает инфу о
    ///текущем пользователе
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_profile("Orhideous");
    pub fn get_profile(&mut self, name: &str) -> Result<UserInfo,TabunError> {
        let name = if name.is_empty() { self.name.clone() } else { name.to_string() };

        let full_url = format!("/profile/{}", name);
        let page = try!(self.get(&full_url));
        let profile = page.find(And(Name("div"),Class("profile")));

        let username = profile.find(And(Name("h2"),Attr("itemprop","nickname")))
            .first()
            .unwrap()
            .text();

        let realname = match profile.find(And(Name("p"),Attr("itemprop","name")))
            .first() {
                Some(x) => x.text(),
                None => String::new()
            };

        let skill_area = profile.find(And(Name("div"),Class("strength")))
            .find(Name("div"))
            .first()
            .unwrap();
        let skill = skill_area
            .text()
            .parse::<f32>()
            .unwrap();

        let user_id = skill_area
            .attr("id")
            .unwrap()
            .split('_')
            .collect::<Vec<_>>()[2]
            .parse::<u32>()
            .unwrap();

        let rating = profile.find(Class("vote-count"))
            .find(Name("span"))
            .first()
            .unwrap()
            .text()
            .parse::<f32>().unwrap();

        let about = page.find(And(Name("div"),Class("profile-info-about")))
            .first()
            .unwrap();

        let userpic = about.find(Class("avatar"))
            .find(Name("img"))
            .first()
            .unwrap();
        let userpic = userpic
            .attr("src")
            .unwrap();

        let description = about.find(And(Name("div"),Class("text")))
            .first()
            .unwrap()
            .inner_html();

        let dotted = page.find(And(Name("ul"), Class("profile-dotted-list")));
        let dotted = dotted.iter().last().unwrap().find(Name("li"));

        let mut other_info = HashMap::<String,String>::new();

        let mut created = Vec::<String>::new();
        let mut admin = Vec::<String>::new();
        let mut moderator = Vec::<String>::new();
        let mut member= Vec::<String>::new();

        for li in dotted.iter() {
            let name = li.find(Name("span")).first().unwrap().text();
            let val = li.find(Name("strong")).first().unwrap();

            if name.contains("Создал"){
                created = val.find(Name("a")).iter().map(|x| x.text()).collect::<Vec<_>>();
            } else if name.contains("Администрирует") {
                admin = val.find(Name("a")).iter().map(|x| x.text()).collect::<Vec<_>>();
            } else if name.contains("Модерирует") {
                moderator = val.find(Name("a")).iter().map(|x| x.text()).collect::<Vec<_>>();
            } else if name.contains("Состоит") {
                member = val.find(Name("a")).iter().map(|x| x.text()).collect::<Vec<_>>();
            } else {
                other_info.insert(name.replace(":",""),val.text());
            }
        }

        let blogs = InBlogs{
            created: created,
            admin: admin,
            moderator: moderator,
            member: member
        };

        let nav = page.find(Class("nav-profile")).find(Name("li"));

        let (mut publications,mut favourites, mut friends) = (0,0,0);

        for li in nav.iter() {
            let a = li.find(Name("a")).first().unwrap().text();

            if !a.contains("Инфо") {
                 let a = a.split('(').collect::<Vec<_>>();
                 if a.len() >1 {
                     let val = a[1].to_string()
                         .replace(")","")
                         .parse::<u32>()
                         .unwrap();
                     if a[0].contains(&"Публикации") {
                         publications = val
                     } else if a[0].contains(&"Избранное") {
                         favourites = val
                     } else {
                         friends = val
                     }
                 }
            }
        }

        Ok(UserInfo{
            username:       username,
            realname:       realname,
            skill:          skill,
            id:             user_id,
            rating:         rating,
            userpic:        userpic.to_owned(),
            description:    description,
            other_info:     other_info,
            blogs:          blogs,
            publications:   publications,
            favourites:     favourites,
            friends:        friends
        })
    }

    ///Добавляет что-то в избранное, true - коммент, false - пост
    fn favourite(&mut self, id: u32, typ: bool, fn_typ: bool) -> Result<u32, TabunError> {
        let id = id.to_string();
        let key = self.security_ls_key.clone();

        let body = map![
        if fn_typ { "idComment"} else { "idTopic" } => id.as_str(),
        "type"                                      => &(if typ { "1" } else { "0" }),
        "security_ls_key"                           => &key
        ];

        let mut res = try!(self.multipart(&format!("/ajax/favourite/{}/", if fn_typ { "comment" } else { "topic" }),body));

        if res.status != hyper::Ok { return Err(TabunError::NumError(res.status)) }

        let mut bd = String::new();
        res.read_to_string(&mut bd).unwrap();

        if bd.contains("\"bStateError\":true") {
            let err = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap().captures(&bd).unwrap();
            Err(TabunError::Error(
                    unescape!(err.at(1).unwrap()),
                    unescape!(err.at(2).unwrap())))
        } else {
            parse_text_to_res!(regex => "\"iCount\":(\\d+)", st => &bd, num => 1, typ => u32)
        }
    }
}

#[test]
fn test_parsetext_macro() {
    let r : Result<u32, regex::Error> = parse_text_to_res!(regex => r"sometext (\d+) sometext", st => "sometext 001 sometext", num => 1, typ => u32);
    match r {
        Ok(x)   => assert_eq!(x, 1),
        Err(_)  => unreachable!()
    }
}

#[test]
fn test_blog_id() {
    let mut user = TClient::new("","").unwrap();
    match user.get_blog_id("lighthouse") {
        Ok(x)   => assert_eq!(15558, x),
        Err(x)  => panic!(x)
    }
}

#[test]
fn test_get_profile() {
    let mut user = TClient::new("","").unwrap();
    match user.get_profile("OrHiDeOuS") {
        Ok(x)   => assert_eq!(x.username, "Orhideous"),
        Err(x)  => panic!(x)
    }
}
