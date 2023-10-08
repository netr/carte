use reqwest::Proxy;

#[derive(Clone)]
pub struct ClientSettings {
    proxy: Option<Proxy>,
    user_agent: Option<String>,
    gzip: bool,
}

impl ClientSettings {
    pub fn new() -> Self {
        Self {
            proxy: None,
            user_agent: None,
            gzip: true,
        }
    }

    pub fn set_proxy(&mut self, proxy: Option<Proxy>) -> &mut Self {
        self.proxy = proxy;
        self
    }

    pub fn proxy(&self) -> Option<&Proxy> {
        self.proxy.as_ref()
    }

    pub fn set_user_agent(&mut self, user_agent: Option<String>) -> &mut Self {
        self.user_agent = user_agent;
        self
    }

    pub fn user_agent(&self) -> Option<&String> {
        self.user_agent.as_ref()
    }

    pub fn enable_compression(&mut self) -> &mut Self {
        self.gzip = true;
        self
    }

    pub fn disable_compression(&mut self) -> &mut Self {
        self.gzip = false;
        self
    }

    pub fn is_compressed(&self) -> bool {
        self.gzip
    }
}
