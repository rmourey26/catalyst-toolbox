use super::api_params::{ApiParams, DEFAULT_PUSHWOOSH_API_URL};
use catalyst_toolbox::notifications::{
    requests::{
        create_message::{
            ContentSettingsBuilder, ContentType, CreateMessage, CreateMessageBuilder, DATETIME_FMT,
        },
        Request, RequestData,
    },
    send::send_create_message,
    Error,
};
use jcli_lib::utils::io;

use reqwest::Url;
use structopt::StructOpt;
use time::OffsetDateTime;

use std::io::Read;
use std::path::PathBuf;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Content {
    /// Path to file with notification message, if not provided will be read from the stdin
    content_path: Option<PathBuf>,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Args {
    #[structopt(flatten)]
    api_params: ApiParams,

    #[structopt(flatten)]
    content_path: Content,

    /// Pushwoosh application code where message will be send
    #[structopt(long)]
    application: String,

    /// Date and time to send notification of format  "Y-m-d H:M"
    #[structopt(long, parse(try_from_str=parse_datetime))]
    send_date: Option<OffsetDateTime>,

    /// Ignore user timezones when sending a message
    #[structopt(long)]
    ignore_user_timezones: bool,

    /// Select an specific campaign to send the message to
    #[structopt(long)]
    campaign: Option<String>,

    /// Filter options as described by pushwhoosh API
    #[structopt(long)]
    filter: Option<String>,

    /// Timezone of send date, for example "America/New_York"
    #[structopt(long)]
    timezone: Option<String>,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Json {
    /// Pushwoosh API url
    #[structopt(long, default_value = DEFAULT_PUSHWOOSH_API_URL)]
    pub api_url: Url,

    /// Path to file with the json representation of the notification,
    /// if not provided will be read from stdin
    #[structopt(flatten)]
    json_path: Content,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum SendNotification {
    /// Push a notification with setup taken from arguments
    FromArgs(Args),
    /// Push an already built notification from a json object
    FromJson(Json),
}

impl Args {
    pub fn exec(self) -> Result<(), Error> {
        let url = self.api_params.api_url.join("createMessage").unwrap();
        let message = self.build_create_message()?;
        let request = Request::new(RequestData::CreateMessageRequest(message));
        let response = send_create_message(url, &request)?;

        println!("{}", serde_json::to_string_pretty(&response)?);
        Ok(())
    }

    pub fn build_create_message(&self) -> Result<CreateMessage, Error> {
        let content: ContentType = serde_json::from_str(&self.content_path.get_content()?)?;
        let mut content_builder = ContentSettingsBuilder::new()
            .with_timezone(self.timezone.clone())
            .with_campaign(self.campaign.clone())
            .with_filter(self.filter.clone())
            .with_ignore_user_timezones(self.ignore_user_timezones)
            .with_content(content);

        if let Some(datetime) = self.send_date {
            content_builder = content_builder.with_send_date(datetime);
        }

        CreateMessageBuilder::new()
            .with_auth(self.api_params.access_token.clone())
            .with_application(self.application.clone())
            .add_content_settings(content_builder.build()?)
            .build()
            .map_err(Into::into)
    }
}

impl Json {
    pub fn exec(self) -> Result<(), Error> {
        let url = self.api_url.join("createMessage").unwrap();
        let message_data: RequestData = serde_json::from_str(&self.json_path.get_content()?)?;
        let request: Request = Request::new(message_data);
        let response = send_create_message(url, &request)?;

        println!("{}", serde_json::to_string_pretty(&response)?);
        Ok(())
    }
}

impl SendNotification {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            SendNotification::FromArgs(args) => args.exec(),
            SendNotification::FromJson(json) => json.exec(),
        }
    }
}

impl Content {
    pub fn get_content(&self) -> Result<String, Error> {
        let mut reader = io::open_file_read(&self.content_path).map_err(Error::FileError)?;
        let mut result = String::new();
        reader.read_to_string(&mut result)?;
        Ok(result)
    }
}

fn parse_datetime(dt: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(dt, &DATETIME_FMT)
}
