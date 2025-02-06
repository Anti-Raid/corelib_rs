use crate::Error;

pub async fn spawn_task(
    reqwest_client: &reqwest::Client,
    spawn: &super::Spawn,
    jobserver_addr: &str,
    jobserver_port: u16,
) -> Result<super::SpawnResponse, Error> {
    let resp = reqwest_client
        .post(format!("{}:{}/spawn", jobserver_addr, jobserver_port))
        .json(spawn)
        .send()
        .await
        .map_err(|e| format!("Failed to initiate task: {}", e))?;

    if resp.status().is_success() {
        Ok(resp.json::<super::SpawnResponse>().await?)
    } else {
        let err_text = resp.text().await?;

        Err(format!("Failed to initiate task: {}", err_text).into())
    }
}
