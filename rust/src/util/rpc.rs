use filecoin_api::polling::PollingState;
//提交状态
use log::{info, trace};
use reqwest::blocking::Client;
use reqwest::blocking::multipart::Form;
use serde::ser::Serialize;
use serde_json::{json, Value};
use serde_json::value::from_value;
//yst add    rust做一个jsonrpc给别的机器
use std::{fs, thread, time};
use std::fs::File;
use std::io::Read;

// 定义一个client以及相应的host的配置文件
lazy_static! {
    static ref REQWEST_CLIENT:Client = Client::new();
    static ref HOST: String = fs::read_to_string("/etc/lotus-remote.conf").unwrap();
}

pub(crate) fn api_upload<F: AsRef<str>>(file: F) -> Result<String, String> {
    let mut f = File::open(file.as_ref()).map_err(|e| format!("{:?}", e))?;
    let mut buf = vec![];
    f.read_to_end(&mut buf).map_err(|e| format!("{:?}", e))?;

    let form = Form::new().file("api_upload", file.as_ref()).map_err(|e| format!("{:?}", e))?;
    //提交给远程upload_file的程序
    //创建/etc/filecoin-webapi.conf，写入远程服务器ip、端口，如：
    // echo “http://111.44.254.172:45102” > /etc/lotus-remote.conf   远程创建/mnt/upload
    let post = REQWEST_CLIENT.post(&format!("{}/sys/upload_file", &*HOST));
    //response 是返回的结果

    let response = post.multipart(form).send()
        .map_err(|e| format!("{:?}", e))?
        .text()
        .map_err(|e| format!("{:?}", e))?;
    let upload_file: Option<String> = serde_json::from_str(&response).map_err(|e| format!("{:?}", e))?;

    upload_file.ok_or("None".to_string())
}

pub(crate) fn api_post<T: Serialize + ?Sized>(path: &str, json: &T) -> Result<Value, String> {
    let post = REQWEST_CLIENT.post(&format!("{}/{}", &*HOST, path));
    let response = post
        .json(json)
        .send()
        .map_err(|e| format!("{:?}", e))?
        .text()
        .map_err(|e| format!("{:?}", e))?;//发送data得到结果
    let value: Value = serde_json::from_str(&response).map_err(|e| format!("{:?}", e))?;

    if value.get("Err").is_some() {
        return Err(format!("{:?}", value));
    }

    return Ok(value);
}


pub(crate) fn api_post_polling<T: Serialize + ?Sized>(path: &str, json: &T) -> Result<Value, String> {
    //pub enum PollingState {
    //     Started(u64),
    //     Pending,
    //     Done(Value),
    //     Removed,
    //     Error(PollingError),
    // }有5个状态分别是开始持续成功删除以及错误
    let state: PollingState = from_value(api_post(path, json)?).map_err(|e| format!("{:?}", e))?;
    info!("api_post_polling request state: {:?}", state);
    let proc_id = match state {
        PollingState::Started(val) => val,
        _ => {
            return Err(format!("api_post_polling response error: {:?}", state));
        }
    };

    loop {
        let poll_state: PollingState =
            from_value(api_post("sys/query_state", &json!(proc_id))?).map_err(|e| format!("{:?}", e))?;
        trace!("api_post_polling poll_state: {:?}", poll_state);
        match poll_state {
            PollingState::Done(result) => return Ok(result),
            PollingState::Pending => {}
            e @ _ => return Err(format!("poll_state error: {:?}", e)),
        }

        // sleep 30s  休息30s
        let time = time::Duration::from_secs(30);
        thread::sleep(time);
    }
}

macro_rules! api_post {
    ($path:literal, $json:expr) => {
        crate::util::rpc::api_post($path, $json);
    };
}

macro_rules! api_post_polling {
    ($path:literal, $json:expr) => {
        crate::util::rpc::api_post_polling($path, $json);
    };
}