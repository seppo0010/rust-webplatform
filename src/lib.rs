#![allow(unused_unsafe)]

extern crate libc;

use std::ffi::{CString, CStr};
use std::{mem, fmt};
use std::str;
use std::borrow::ToOwned;
use std::ops::Deref;
use std::cell::RefCell;
use std::clone::Clone;
use std::rc::Rc;
use std::collections::{HashSet, HashMap};
use std::char;
use std::iter::IntoIterator;
use std::string::FromUtf8Error;

mod webplatform {
    pub use emscripten_asm_const;
    pub use emscripten_asm_const_int;
}

pub trait Interop {
    fn as_int(self, _:&mut Vec<CString>) -> libc::c_int;
}

impl Interop for i32 {
    fn as_int(self, _:&mut Vec<CString>) -> libc::c_int {
        return self;
    }
}

impl<'a> Interop for &'a str {
    fn as_int(self, arena:&mut Vec<CString>) -> libc::c_int {
        let c = CString::new(self).unwrap();
        let ret = c.as_ptr() as libc::c_int;
        arena.push(c);
        return ret;
    }
}

impl<'a> Interop for *const libc::c_void {
    fn as_int(self, _:&mut Vec<CString>) -> libc::c_int {
        return self as libc::c_int;
    }
}

#[macro_export]
macro_rules! js {
    ( ($( $x:expr ),*) $y:expr ) => {
        {
            let mut arena:Vec<CString> = Vec::new();
            const LOCAL: &'static [u8] = $y;
            unsafe { ::webplatform::emscripten_asm_const_int(&LOCAL[0] as *const _ as *const libc::c_char, $(Interop::as_int($x, &mut arena)),*) }
        }
    };
    ( $y:expr ) => {
        {
            const LOCAL: &'static [u8] = $y;
            unsafe { ::webplatform::emscripten_asm_const_int(&LOCAL[0] as *const _ as *const libc::c_char) }
        }
    };
}

extern "C" {
    pub fn emscripten_asm_con(s: *const libc::c_char);
    pub fn emscripten_asm_const(s: *const libc::c_char);
    pub fn emscripten_asm_const_int(s: *const libc::c_char, ...) -> libc::c_int;
    pub fn emscripten_pause_main_loop();
    pub fn emscripten_set_main_loop(m: extern fn(), fps: libc::c_int, infinite: libc::c_int);
}

#[derive(Debug, Clone)]
pub struct XmlHttpRequest {
    id: libc::c_int,
    pub response: Vec<u8>,
    pub status: u16,
}

#[derive(Debug, Clone)]
pub enum XmlHttpRequestSuccessStatus {
    OK,
    Created,
    NoContent,
    PartialContent,
}

#[derive(Debug, Clone)]
pub enum XmlHttpRequestRedirectStatus {
    MovedPermanently(Option<String>),
    Found(Option<String>),
    SeeOther(Option<String>),
    NotModified,
    TemporaryRedirect(Option<String>),
    PermanentRedirect(Option<String>),
}

#[derive(Debug, Clone)]
pub enum XmlHttpRequestClientErrorStatus {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    NotAcceptable,
}

#[derive(Debug, Clone)]
pub enum XmlHttpRequestServerErrorStatus {
    InternalServerError,
    NotImplemented,
    BadGateway,
    ServiceNotAvailable,
}

#[derive(Debug, Clone)]
pub enum XmlHttpRequestOk {
    XmlHttpRequestSuccess(XmlHttpRequestSuccessStatus),
    XmlHttpRequestRedirect(XmlHttpRequestRedirectStatus),
    Other(u16, Option<String>),
}

impl XmlHttpRequestOk {
    fn from_status(status: u16, url: Option<String>) -> XmlHttpRequestOk {
        match status {
            200 => XmlHttpRequestOk::XmlHttpRequestSuccess(XmlHttpRequestSuccessStatus::OK),
            201 => XmlHttpRequestOk::XmlHttpRequestSuccess(XmlHttpRequestSuccessStatus::Created),
            204 => XmlHttpRequestOk::XmlHttpRequestSuccess(XmlHttpRequestSuccessStatus::NoContent),
            206 => XmlHttpRequestOk::XmlHttpRequestSuccess(XmlHttpRequestSuccessStatus::PartialContent),
            301 => XmlHttpRequestOk::XmlHttpRequestRedirect(XmlHttpRequestRedirectStatus::MovedPermanently(url)),
            302 => XmlHttpRequestOk::XmlHttpRequestRedirect(XmlHttpRequestRedirectStatus::Found(url)),
            303 => XmlHttpRequestOk::XmlHttpRequestRedirect(XmlHttpRequestRedirectStatus::SeeOther(url)),
            304 => XmlHttpRequestOk::XmlHttpRequestRedirect(XmlHttpRequestRedirectStatus::NotModified),
            307 => XmlHttpRequestOk::XmlHttpRequestRedirect(XmlHttpRequestRedirectStatus::TemporaryRedirect(url)),
            308 => XmlHttpRequestOk::XmlHttpRequestRedirect(XmlHttpRequestRedirectStatus::PermanentRedirect(url)),
            status => XmlHttpRequestOk::Other(status, url),
        }
    }
}

#[derive(Debug, Clone)]
pub enum XmlHttpRequestErr {
    XmlHttpRequestClientError(XmlHttpRequestClientErrorStatus),
    XmlHttpRequestServerError(XmlHttpRequestServerErrorStatus),
    Other(u16),
}

impl XmlHttpRequestErr {
    fn from_status(status: u16) -> XmlHttpRequestErr {
        match status {
            400 => XmlHttpRequestErr::XmlHttpRequestClientError(XmlHttpRequestClientErrorStatus::BadRequest),
            401 => XmlHttpRequestErr::XmlHttpRequestClientError(XmlHttpRequestClientErrorStatus::Unauthorized),
            403 => XmlHttpRequestErr::XmlHttpRequestClientError(XmlHttpRequestClientErrorStatus::Forbidden),
            404 => XmlHttpRequestErr::XmlHttpRequestClientError(XmlHttpRequestClientErrorStatus::NotFound),
            405 => XmlHttpRequestErr::XmlHttpRequestClientError(XmlHttpRequestClientErrorStatus::MethodNotAllowed),
            406 => XmlHttpRequestErr::XmlHttpRequestClientError(XmlHttpRequestClientErrorStatus::NotAcceptable),
            500 => XmlHttpRequestErr::XmlHttpRequestServerError(XmlHttpRequestServerErrorStatus::InternalServerError),
            501 => XmlHttpRequestErr::XmlHttpRequestServerError(XmlHttpRequestServerErrorStatus::NotImplemented),
            502 => XmlHttpRequestErr::XmlHttpRequestServerError(XmlHttpRequestServerErrorStatus::BadGateway),
            503 => XmlHttpRequestErr::XmlHttpRequestServerError(XmlHttpRequestServerErrorStatus::ServiceNotAvailable),
            status => XmlHttpRequestErr::Other(status),
        }
    }
}

impl XmlHttpRequest {
    pub fn as_result(&self) -> Result<XmlHttpRequestOk, XmlHttpRequestErr> {
        if self.status >= 400 {
            Err(XmlHttpRequestErr::from_status(self.status))
        } else {
            Ok(XmlHttpRequestOk::from_status(self.status, self.get_location()))
        }
    }

    pub fn response_text(&self) -> Result<String, FromUtf8Error>{
        String::from_utf8(self.response.clone())

    }

    fn get_location(&self) -> Option<String> {
        let a = js! { (self.id) b"\
            var str = WEBPLATFORM.rs_refs[$0].getResponseHeader('Location');\
            if (str == null) return -1;\
            return allocate(intArrayFromString(str), 'i8', ALLOC_STACK);\
        \0" };
        if a == -1 {
            None
        } else {
            Some(unsafe {
                str::from_utf8(CStr::from_ptr(a as *const libc::c_char).to_bytes()).unwrap().to_owned()
            })
        }
    }
}

pub struct HtmlNode<'a> {
    id: libc::c_int,
    doc: *const Document<'a>,
}

impl<'a> fmt::Debug for HtmlNode<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "HtmlNode({:?})", self.id)
    }
}

impl<'a> Drop for HtmlNode<'a> {
    fn drop(&mut self) {
        println!("dropping HTML NODE {:?}", self.id);
    }
}

pub struct JSRef<'a> {
    ptr: *const HtmlNode<'a>,
}

impl<'a> fmt::Debug for JSRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "JSRef(HtmlNode({:?}))", self.id)
    }
}

impl<'a> Clone for JSRef<'a> {
    fn clone(&self) -> JSRef<'a> {
        JSRef {
            ptr: self.ptr,
        }
    }
}

impl<'a> HtmlNode<'a> {
    pub fn root_ref(&self) -> JSRef<'a> {
        JSRef {
            ptr: &*self,
        }
    }
}

impl<'a> Deref for JSRef<'a> {
    type Target = HtmlNode<'a>;

    fn deref(&self) -> &HtmlNode<'a> {
        unsafe {
            &*self.ptr
        }
    }
}

pub struct Event<'a> {
    pub target: Option<HtmlNode<'a>>,
    pub event: i32,
}

impl<'a> Event<'a> {
    pub fn prevent_default(&self) {
        js! { (self.event) b"\
            WEBPLATFORM.rs_refs[$0].preventDefault();\
        \0" };
    }
}

extern fn event_rust_caller<F: FnMut(Event) + Sync + Send>(a: *const libc::c_void, docptr: *const libc::c_void, id: i32, event: i32) {
    let v:&mut F = unsafe { mem::transmute(a) };
    v(Event {
        event: event,
        target: if id == -1 {
            None
        } else {
            Some(HtmlNode {
                id: id,
                doc: unsafe { mem::transmute(docptr) },
            })
        }
        // target: None,
    });
}

extern fn ajax_rust_caller<'a, F: FnMut(XmlHttpRequest) + Sync + Send>(a: *const libc::c_void, id: libc::c_int, response: *const libc::c_void, status: u16, doc: *const Document<'a>) {
    let v:&mut F = unsafe { mem::transmute(a) };
    let r = unsafe {
        CStr::from_ptr(response as *const libc::c_char).to_bytes().to_vec()
    };
    let mut request = unsafe {
        (&*doc).requests.borrow_mut().remove(&id).unwrap()
    };
    request.status = status;
    request.response = r;
    v(request);
}

fn ajax<'a, F: FnMut(XmlHttpRequest) + Sync + Send + 'a>(doc: *const Document<'a>, url: &str, method: &str, data: Option<&str>, f: F) {
    let request = XmlHttpRequest {
        id: js! { b"\
            var request = new XMLHttpRequest();\
            return WEBPLATFORM.rs_refs.push(request) - 1;\
        \0" },
        response: Vec::new(),
        status: 0,
    };

    let b = Box::new(f);
    let a = &*b as *const _;
    js! { (request.id, url, method,
        a as *const libc::c_void,
        ajax_rust_caller::<F> as *const libc::c_void,
        doc as *const libc::c_void,
        data.unwrap_or("")
    ) b"\
    var request = WEBPLATFORM.rs_refs[$0];\
    request.open(UTF8ToString($2), UTF8ToString($1), true);\
    var tostr = function(s) { return allocate(intArrayFromString(s), 'i8', ALLOC_STACK); };\
    request.onload = function() {\
        Runtime.dynCall('viiiii', $4, [$3, $0, tostr(request.responseText), request.status, $5])\
    };\
    request.onerror = function() {\
        Runtime.dynCall('viiiii', $4, [$3, $0, tostr('error'), 65535, $5])\
    };\
    var data = UTF8ToString($6);\
    request.send(data ? data : null);\
    \0" };

    unsafe {
        (&*doc).requests.borrow_mut().insert(request.id, request);
        (&*doc).ajax.borrow_mut().push(b);
    }
}

pub fn ajax_get<'a, F: FnMut(XmlHttpRequest) + Sync + Send + 'a>(doc: *const Document<'a>, url: &str, f: F) {
    ajax(doc, url, "GET", None, f)
}

pub fn ajax_post<'a, F: FnMut(XmlHttpRequest) + Sync + Send + 'a>(doc: *const Document<'a>, url: &str, data: Option<&str>, f: F) {
    ajax(doc, url, "POST", data, f)
}

impl<'a> HtmlNode<'a> {
    pub fn tagname(&self) -> String {
        let a = js! { (self.id) b"\
            var str = WEBPLATFORM.rs_refs[$0].tagName.toLowerCase();\
            return allocate(intArrayFromString(str), 'i8', ALLOC_STACK);\
        \0" };
        unsafe {
            str::from_utf8(CStr::from_ptr(a as *const libc::c_char).to_bytes()).unwrap().to_owned()
        }
    }

    pub fn focus(&self) {
        js! { (self.id) b"\
            WEBPLATFORM.rs_refs[$0].focus();\
        \0" };
    }

    pub fn html_set(&self, s: &str) {
        js! { (self.id, s) b"\
            WEBPLATFORM.rs_refs[$0].innerHTML = UTF8ToString($1);\
        \0" };
    }

    pub fn html_get(&self) -> String {
        let a = js! { (self.id) b"\
            return allocate(intArrayFromString(WEBPLATFORM.rs_refs[$0].innerHTML), 'i8', ALLOC_STACK);\
        \0" };
        unsafe {
            str::from_utf8(CStr::from_ptr(a as *const libc::c_char).to_bytes()).unwrap().to_owned()
        }
    }

    pub fn class_get(&self) -> HashSet<String> {
        let a = js! { (self.id) b"\
            return allocate(intArrayFromString(WEBPLATFORM.rs_refs[$0].className), 'i8', ALLOC_STACK);\
        \0" };
        let class = unsafe {
            str::from_utf8(CStr::from_ptr(a as *const libc::c_char).to_bytes()).unwrap().to_owned()
        };
        class.trim().split(char::is_whitespace).map(|x| x.to_string()).collect()
    }

    pub fn class_add(&self, s: &str) {
        js! { (self.id, s) b"\
            WEBPLATFORM.rs_refs[$0].classList.add(UTF8ToString($1));\
        \0" };
    }

    pub fn class_remove(&self, s: &str) {
        js! { (self.id, s) b"\
            WEBPLATFORM.rs_refs[$0].classList.remove(UTF8ToString($1));\
        \0" };
    }

    pub fn parent(&self) -> Option<HtmlNode<'a>> {
        let id = js! { (self.id) b"\
            var value = WEBPLATFORM.rs_refs[$0].parentNode;\
            if (!value) {\
                return -1;\
            }\
            return WEBPLATFORM.rs_refs.push(value) - 1;\
        \0" };
        if id < 0 {
            None
        } else {
            Some(HtmlNode {
                id: id,
                doc: self.doc,
            })
        }
    }

    pub fn data_set(&self, s: &str, v: &str) {
        js! { (self.id, s, v) b"\
            WEBPLATFORM.rs_refs[$0].dataset[UTF8ToString($1)] = UTF8ToString($2);\
        \0" };
    }

    pub fn data_get(&self, s: &str) -> Option<String> {
        let a = js! { (self.id, s) b"\
            var str = WEBPLATFORM.rs_refs[$0].dataset[UTF8ToString($1)];\
            if (str == null) return -1;\
            return allocate(intArrayFromString(str), 'i8', ALLOC_STACK);\
        \0" };
        if a == -1 {
            None
        } else {
            Some(unsafe {
                str::from_utf8(CStr::from_ptr(a as *const libc::c_char).to_bytes()).unwrap().to_owned()
            })
        }
    }

    pub fn style_set_str(&self, s: &str, v: &str) {
        js! { (self.id, s, v) b"\
            WEBPLATFORM.rs_refs[$0].style[UTF8ToString($1)] = UTF8ToString($2);\
        \0" };
    }

    pub fn style_get_str(&self, s: &str) -> String {
        let a = js! { (self.id, s) b"\
            return allocate(intArrayFromString(WEBPLATFORM.rs_refs[$0].style[UTF8ToString($1)]), 'i8', ALLOC_STACK);\
        \0" };
        unsafe {
            str::from_utf8(CStr::from_ptr(a as *const libc::c_char).to_bytes()).unwrap().to_owned()
        }
    }

    pub fn prop_set_i32(&self, s: &str, v: i32) {
        js! { (self.id, s, v) b"\
            WEBPLATFORM.rs_refs[$0][UTF8ToString($1)] = $2;\
        \0" };
    }

    pub fn prop_set_str(&self, s: &str, v: &str) {
        js! { (self.id, s, v) b"\
            WEBPLATFORM.rs_refs[$0][UTF8ToString($1)] = UTF8ToString($2);\
        \0" };
    }

    pub fn prop_del(&self, s: &str) {
        js! { (self.id, s) b"\
            delete WEBPLATFORM.rs_refs[$0][UTF8ToString($1)];\
        \0" };
    }

    pub fn prop_get_i32(&self, s: &str) -> i32 {
        return js! { (self.id, s) b"\
            return Number(WEBPLATFORM.rs_refs[$0][UTF8ToString($1)])\
        \0" };
    }

    pub fn prop_get_str(&self, s: &str) -> String {
        let a = js! { (self.id, s) b"\
            var a = allocate(intArrayFromString(WEBPLATFORM.rs_refs[$0][UTF8ToString($1)]), 'i8', ALLOC_STACK); console.log(WEBPLATFORM.rs_refs[$0]); return a;\
        \0" };
        unsafe {
            str::from_utf8(CStr::from_ptr(a as *const libc::c_char).to_bytes()).unwrap().to_owned()
        }
    }

    pub fn append(&self, s: &HtmlNode) {
        js! { (self.id, s.id) b"\
            WEBPLATFORM.rs_refs[$0].appendChild(WEBPLATFORM.rs_refs[$1]);\
        \0" };
    }

    pub fn html_append(&self, s: &str) {
        js! { (self.id, s) b"\
            WEBPLATFORM.rs_refs[$0].insertAdjacentHTML('beforeEnd', UTF8ToString($1));\
        \0" };
    }

    pub fn html_prepend(&self, s: &str) {
        js! { (self.id, s) b"\
            WEBPLATFORM.rs_refs[$0].insertAdjacentHTML('afterBegin', UTF8ToString($1));\
        \0" };
    }

    pub fn on<F: FnMut(Event) + Sync + Send + 'a>(&self, s: &str, f: F) {
        unsafe {
            let b = Box::new(f);
            let a = &*b as *const _;
            js! { (self.id, s, a as *const libc::c_void,
                event_rust_caller::<F> as *const libc::c_void,
                self.doc as *const libc::c_void)
                b"\
                WEBPLATFORM.rs_refs[$0].addEventListener(UTF8ToString($1), function (e) {\
                    Runtime.dynCall('viiii', $3, [$2, $4, e.target ? WEBPLATFORM.rs_refs.push(e.target) - 1 : -1, WEBPLATFORM.rs_refs.push(e) - 1]);\
                }, false);\
            \0" };
            (&*self.doc).refs.borrow_mut().push(b);
        }
    }

    pub fn captured_on<F: FnMut(Event) + Sync + Send + 'a>(&self, s: &str, f: F) {
        unsafe {
            let b = Box::new(f);
            let a = &*b as *const _;
            js! { (self.id, s, a as *const libc::c_void,
                event_rust_caller::<F> as *const libc::c_void,
                self.doc as *const libc::c_void)
                b"\
                WEBPLATFORM.rs_refs[$0].addEventListener(UTF8ToString($1), function (e) {\
                    Runtime.dynCall('viii', $3, [$2, $4, e.target ? WEBPLATFORM.rs_refs.push(e.target) - 1 : -1]);\
                }, true);\
            \0" };
            (&*self.doc).refs.borrow_mut().push(b);
        }
    }

    pub fn remove_self(&self) {
        js! { (self.id) b"\
            var s = WEBPLATFORM.rs_refs[$0];\
            s.parentNode.removeChild(s);\
        \0" };
    }
}

pub fn alert(s: &str) {
    js! { (s) b"\
        alert(UTF8ToString($0));\
    \0" };
}

pub struct Document<'a> {
    refs: Rc<RefCell<Vec<Box<FnMut(Event<'a>) + Sync + Send + 'a>>>>,
    ajax: Rc<RefCell<Vec<Box<FnMut(XmlHttpRequest) + Sync + Send + 'a>>>>,
    requests: Rc<RefCell<HashMap<i32, XmlHttpRequest>>>,
}

unsafe impl<'a> Sync for Document<'a> {}
unsafe impl<'a> Send for Document<'a> {}

impl<'a> Document<'a> {
    pub fn element_create<'b>(&'b self, s: &str) -> Option<HtmlNode<'a>> {
        let id = js! { (s) b"\
            var value = document.createElement(UTF8ToString($0));\
            if (!value) {\
                return -1;\
            }\
            return WEBPLATFORM.rs_refs.push(value) - 1;\
        \0" };

        if id < 0 {
            None
        } else {
            Some(HtmlNode {
                id: id,
                doc: &*self,
            })
        }
    }

    pub fn location_hash_get(&self) -> String {
        let a = js! { b"\
            return allocate(intArrayFromString(window.location.hash), 'i8', ALLOC_STACK);\
        \0" };
        unsafe {
            str::from_utf8(CStr::from_ptr(a as *const libc::c_char).to_bytes()).unwrap().to_owned()
        }
    }

    pub fn on<F: FnMut(Event) + Sync + Send + 'a>(&self, s: &str, f: F) {
        unsafe {
            let b = Box::new(f);
            let a = &*b as *const _;
            js! { (0, s, a as *const libc::c_void,
                event_rust_caller::<F> as *const libc::c_void,
                &*self as *const _ as *const libc::c_void)
                b"\
                window.addEventListener(UTF8ToString($1), function (e) {\
                    Runtime.dynCall('viii', $3, [$2, $4, e.target ? WEBPLATFORM.rs_refs.push(e.target) - 1 : -1]);\
                }, false);\
            \0" };
            self.refs.borrow_mut().push(b);
        }
    }

    pub fn element_query<'b>(&'b self, s: &str) -> Option<HtmlNode<'a>> {
        let id = js! { (s) b"\
            var value = document.querySelector(UTF8ToString($0));\
            if (!value) {\
                return -1;\
            }\
            return WEBPLATFORM.rs_refs.push(value) - 1;\
        \0" };

        if id < 0 {
            None
        } else {
            Some(HtmlNode {
                id: id,
                doc: self,
            })
        }
    }
}

pub struct LocalStorageInterface;

pub struct LocalStorageIterator {
    index: i32,
}

impl LocalStorageInterface {
    pub fn len(&self) -> i32 {
        js! { b"\
            return window.localStorage.length;\
        \0" }
    }

    pub fn clear(&self) {
        js! { b"\
            window.localStorage.clear();\
        \0" };
    }

    pub fn remove(&self, s: &str) {
        js! { (s) b"\
            window.localStorage.removeItem(UTF8ToString($0));\
        \0" };
    }

    pub fn set(&self, s: &str, v: &str) {
        js! { (s, v) b"\
            window.localStorage.setItem(UTF8ToString($0), UTF8ToString($1));\
        \0" };
    }

    pub fn get(&self, name: &str) -> Option<String> {
        let a = js! { (name) b"\
            var str = window.localStorage.getItem(UTF8ToString($0));\
            if (str == null) {\
                return -1;\
            }\
            return allocate(intArrayFromString(str), 'i8', ALLOC_STACK);\
        \0" };
        if a == -1 {
            None
        } else {
            Some(unsafe {
                str::from_utf8(CStr::from_ptr(a as *const libc::c_char).to_bytes()).unwrap().to_owned()
            })
        }
    }

    pub fn key(&self, index: i32) -> String {
        let a = js! { (index) b"\
            var key = window.localStorage.key($0);\
            return allocate(intArrayFromString(str), 'i8', ALLOC_STACK);\
        \0" };
        unsafe {
            str::from_utf8(CStr::from_ptr(a as *const libc::c_char).to_bytes()).unwrap().to_owned()
        }
    }
}

impl IntoIterator for LocalStorageInterface {
    type Item = String;
    type IntoIter = LocalStorageIterator;

    fn into_iter(self) -> LocalStorageIterator {
        LocalStorageIterator { index: 0 }
    }
}

impl Iterator for LocalStorageIterator {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        if self.index >= LocalStorage.len() {
            None
        } else {
            LocalStorage.get(&LocalStorage.key(self.index))
        }
    }
}

#[allow(non_upper_case_globals)]
pub const LocalStorage: LocalStorageInterface = LocalStorageInterface;

pub fn init<'a>() -> Document<'a> {
    js! { b"\
        console.log('hi');\
        window.WEBPLATFORM || (window.WEBPLATFORM = {\
            rs_refs: [],\
        });\
    \0" };
    Document {
        refs: Rc::new(RefCell::new(Vec::new())),
        ajax: Rc::new(RefCell::new(Vec::new())),
        requests: Rc::new(RefCell::new(HashMap::new())),
    }
}

extern fn leavemebe() {
    unsafe {
        emscripten_pause_main_loop();
    }
}

pub fn spin() {
    unsafe {
        emscripten_set_main_loop(leavemebe, 0, 1);

    }
}

#[no_mangle]
pub extern "C" fn syscall(a: i32) -> i32 {
    if a == 355 {
        return 55
    }
    return -1
}
