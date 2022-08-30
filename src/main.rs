use openssh::*;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use yaml_rust::YamlLoader;

#[derive(Debug)]
struct Config {
    pre_commands: Vec<String>,
    target_username: String,
    target_host: String,
    target_location: String,
    target_command: String,
}

impl Config {
    fn new(path: &str) -> Self {
        let mut file = File::open(path).unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data)
            .expect("Error while reading file");
        let config = YamlLoader::load_from_str(&data).unwrap();
        let mut pre_commands: Vec<String> = Vec::new();
        for command in config[0]["pre_command"].clone() {
            pre_commands.push(command.as_str().unwrap().to_string())
        }
        Self {
            target_host: config[0]["target_host"].as_str().unwrap().to_string(),
            target_location: config[0]["target_location"].as_str().unwrap().to_string(),
            target_username: config[0]["target_username"].as_str().unwrap().to_string(),
            target_command: config[0]["target_command"].as_str().unwrap().to_string(),
            pre_commands,
        }
    }

    fn run_pre_commands(&self) {
        for command in &self.pre_commands {
            println!("[RUNNING] {}", command);
            let commands: Vec<&str> = command.split(' ').collect();
            let mut cmd = Command::new(commands[0])
                .args(&commands[1..])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
            let status = cmd.wait().unwrap();
            println!("[SUCCESS] exit code: {}", status.code().unwrap());
        }
    }

    async fn login_to_host(&self) -> Result<(), Box<dyn std::error::Error>> {
        let session = Session::connect(
            format!("ssh://{}@{}", self.target_username, self.target_host),
            KnownHosts::Strict,
        )
        .await
        .expect("Cannot login to host");
        let whoami = session.command("whoami").output().await.unwrap();
        assert_eq!(
            whoami.stdout,
            format!("{}\n", self.target_username).as_bytes()
        );
        println!("SSH login successful!");
        session
            .shell(format!(
                "cd {} && {}",
                self.target_location, self.target_command,
            ))
            .spawn()
            .await?
            .wait()
            .await?;
        session.close().await?;
        Ok(())
    }

    async fn do_rsync_with_host(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::new("rsync")
            .arg("-rCP")
            .arg(".")
            .arg(format!(
                "{}@{}:~/{}",
                self.target_username, self.target_host, self.target_location
            ))
            .arg("--delete")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;
        let status = cmd.wait().unwrap();
        if status.code().unwrap() == 0 {
            println!("rsync successful!");
            return Ok(());
        }
        println!("rsync failure!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new("config.yaml");
    config.run_pre_commands();
    config.do_rsync_with_host().await?;
    config.login_to_host().await?;
    Ok(())
}
