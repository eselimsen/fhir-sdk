//! Builder implementation for the client.

use std::marker::PhantomData;

use reqwest::Url;

use super::{auth::AuthCallback, Client, Error, LoginManager, RequestSettings};

/// Default user agent of this client.
const DEFAULT_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Builder for the [Client]
pub struct ClientBuilder<Version = super::DefaultVersion, ACB = ()> {
	/// The FHIR server's base URL.
	base_url: Option<Url>,
	/// Reqwest Client
	client: Option<reqwest::Client>,
	/// User agent to use for requests.
	user_agent: Option<String>,
	/// Request settings.
	request_settings: Option<RequestSettings>,
	/// Auth callback.
	auth_callback: Option<ACB>,
	/// FHIR version.
	version: PhantomData<Version>,
}

impl<V, ACB> Default for ClientBuilder<V, ACB> {
	fn default() -> Self {
		Self {
			base_url: None,
			client: None,
			user_agent: None,
			request_settings: None,
			auth_callback: None,
			version: PhantomData,
		}
	}
}

impl<V, ACB> ClientBuilder<V, ACB> {
	/// The FHIR server's base URL.
	#[must_use]
	pub fn base_url(mut self, base_url: Url) -> Self {
		self.base_url = Some(base_url);
		self
	}

	/// Reqwest client
	#[must_use]
	pub fn client(mut self, client: reqwest::Client) -> Self {
		self.client = Some(client);
		self
	}

	/// User agent to use for requests. This is ignored if a Reqwest client is
	/// passed to the builder using the client() method.
	#[must_use]
	pub fn user_agent(mut self, user_agent: String) -> Self {
		self.user_agent = Some(user_agent);
		self
	}

	/// Request settings.
	#[must_use]
	pub fn request_settings(mut self, settings: RequestSettings) -> Self {
		self.request_settings = Some(settings);
		self
	}

	/// Set an authorization callback to be called every time the server returns
	/// unauthorized. Should return the header value for the `Authorization`
	/// header.
	///
	/// Valid login managers are:
	/// - Async functions `async fn my_auth_callback(client: reqwest::Client) ->
	///   Result<HeaderValue, MyError>`
	/// - Async closures `|client: reqwest::Client| async move { ... }`
	/// - Types that implement [LoginManager]
	///
	/// Calling this with unit type `()` will panic on auth_callback called.
	/// `()` is allowed at compile time for convenience reasons (generics
	/// stuff).
	#[must_use]
	pub fn auth_callback<LM>(self, login_manager: LM) -> ClientBuilder<V, LM>
	where
		LM: LoginManager + 'static,
	{
		ClientBuilder {
			base_url: self.base_url,
			client: self.client,
			user_agent: self.user_agent,
			request_settings: self.request_settings,
			auth_callback: Some(login_manager),
			version: self.version,
		}
	}

	/// Finalize building the client.
	pub fn build(self) -> Result<Client<V>, Error>
	where
		ACB: LoginManager + 'static,
	{
		let Some(base_url) = self.base_url else {
			return Err(Error::BuilderMissingField("base_url"));
		};
		if base_url.cannot_be_a_base() {
			return Err(Error::UrlCannotBeBase);
		}

		let client = match self.client {
			Some(client) => client,
			None => {
				let user_agent = self.user_agent.as_deref().unwrap_or(DEFAULT_USER_AGENT);
				reqwest::Client::builder().user_agent(user_agent).build()?
			}
		};

		let request_settings = self.request_settings.unwrap_or_default();

		let data = super::ClientData {
			base_url,
			client,
			request_settings: std::sync::Mutex::new(request_settings),
			auth_callback: tokio::sync::Mutex::new(self.auth_callback.map(AuthCallback::new)),
		};
		Ok(Client::from(data))
	}
}

impl<V, ACB> Clone for ClientBuilder<V, ACB>
where
	ACB: Clone,
{
	fn clone(&self) -> Self {
		Self {
			base_url: self.base_url.clone(),
			client: self.client.clone(),
			user_agent: self.user_agent.clone(),
			request_settings: self.request_settings.clone(),
			auth_callback: self.auth_callback.clone(),
			version: self.version,
		}
	}
}

impl<V, ACB> std::fmt::Debug for ClientBuilder<V, ACB> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ClientBuilder")
			.field("base_url", &self.base_url)
			.field("user_agent", &self.user_agent)
			.field("request_settings", &self.request_settings)
			.field("auth_callback", &self.auth_callback.as_ref().map(|_| "<login_manager>"))
			.finish()
	}
}
