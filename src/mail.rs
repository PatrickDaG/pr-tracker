use std::collections::HashSet;

use anyhow::Result;
use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::env;

pub fn send_notification(
    recipient: &str,
    branches: &HashSet<String>,
    pr_number: &str,
    pr_title: &str,
    last: bool,
) -> Result<()> {
    let mut body = format!(
        "This is your friendly neighbourhood pr-tracker.\n\
        PR Number {pr_number} titled: \"{pr_title}\" has reached:\n\
        {:#?}\n",
        branches
    );
    if last {
        body += "This is the last update you will get for this pr.\n\
        Thx for using this service\n\
        Goodbye";
    }
    let sending_address = env::var("PR_TRACKER_MAIL_ADDRESS")?;
    let sending_user = match env::var("PR_TRACKER_MAIL_USER") {
        Ok(address) => address,
        _ => sending_address.clone(),
    };
    let sending_passwd = env::var("PR_TRACKER_MAIL_PASSWD")?;

    let sending_server = env::var("PR_TRACKER_MAIL_SERVER")?;

    let email = Message::builder()
        .from(format!("PR-Tracker <{}>", sending_address).parse()?)
        .to(Mailbox::new(None, recipient.parse()?))
        .subject(format!(
            "PR-tracker: {pr_number}: {pr_title} has reached {:?}",
            branches
        ))
        .header(ContentType::TEXT_PLAIN)
        .body(body)?;

    let creds = Credentials::new(sending_user.to_string(), sending_passwd.to_string());

    // Open a remote connection to gmail
    let mailer = SmtpTransport::relay(&sending_server)?
        .credentials(creds)
        .build();

    // Send the email
    mailer.send(&email)?;

    println!("Email sent successfully!");
    Ok(())
}
