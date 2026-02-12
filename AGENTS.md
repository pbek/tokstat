# AI Agents Support in tokstat

This document describes how to add support for various AI agent platforms and services to tokstat.

## Overview

tokstat is designed to monitor token quotas across multiple AI providers. While the initial implementation focuses on GitHub Copilot and OpenRouter, the architecture is designed to support various AI agent platforms including:

- Autonomous agent frameworks
- AI coding assistants
- Language model API providers
- AI agent orchestration platforms

## Current Provider Support

### 1. GitHub Copilot

- **Type**: AI Coding Assistant
- **Authentication**: OAuth 2.0 device flow
- **Metrics**: Token usage, request count
- **Status**: ✅ Implemented

### 2. OpenRouter

- **Type**: LLM API Aggregator
- **Authentication**: API key
- **Metrics**: Cost tracking, credit limits
- **Status**: ✅ Implemented

## Potential AI Agent Platforms

### Autonomous Agent Frameworks

#### AutoGPT

- **Authentication**: API key or self-hosted
- **Metrics**: Task execution count, token usage, cost
- **API**: Self-hosted or cloud service
- **Implementation complexity**: Medium

#### LangChain

- **Authentication**: Depends on backend (OpenAI, Anthropic, etc.)
- **Metrics**: Chain execution count, token usage per chain
- **API**: Via LangSmith API
- **Implementation complexity**: Medium-High

#### BabyAGI

- **Authentication**: Typically uses OpenAI API keys
- **Metrics**: Task queue depth, execution count
- **API**: Via underlying LLM provider
- **Implementation complexity**: Low-Medium

### AI Coding Assistants

#### Cursor

- **Authentication**: API key or OAuth
- **Metrics**: Code generation tokens, requests
- **API**: Private (may require partnership)
- **Implementation complexity**: High (requires API access)

#### Tabnine

- **Authentication**: API key
- **Metrics**: Completion count, team usage
- **API**: Tabnine API
- **Implementation complexity**: Medium

#### Codeium

- **Authentication**: API key
- **Metrics**: Completion count, token usage
- **API**: Codeium API
- **Implementation complexity**: Medium

### LLM API Providers

#### OpenAI

- **Authentication**: API key
- **Metrics**: Token usage, cost, rate limits
- **API**: https://api.openai.com/v1/usage
- **Implementation complexity**: Low
- **Priority**: HIGH

#### Anthropic Claude

- **Authentication**: API key
- **Metrics**: Token usage, cost
- **API**: https://api.anthropic.com/v1/usage
- **Implementation complexity**: Low
- **Priority**: HIGH

#### Google AI (Gemini)

- **Authentication**: API key
- **Metrics**: Token usage, request count
- **API**: Google Cloud API
- **Implementation complexity**: Medium

#### Cohere

- **Authentication**: API key
- **Metrics**: Token usage, model-specific quotas
- **API**: Cohere API
- **Implementation complexity**: Low

#### Mistral AI

- **Authentication**: API key
- **Metrics**: Token usage, cost
- **API**: Mistral API
- **Implementation complexity**: Low

### Agent Orchestration Platforms

#### LangGraph

- **Authentication**: Via LangSmith
- **Metrics**: Graph execution count, token usage
- **API**: LangSmith API
- **Implementation complexity**: Medium

#### Crew AI

- **Authentication**: API key
- **Metrics**: Crew execution count, agent tasks
- **API**: CrewAI Cloud API
- **Implementation complexity**: Medium

#### Semantic Kernel

- **Authentication**: Azure AD or API key
- **Metrics**: Function execution count, token usage
- **API**: Azure OpenAI API
- **Implementation complexity**: Medium-High

### AI Development Platforms

#### Hugging Face

- **Authentication**: API token
- **Metrics**: Inference API usage, model downloads
- **API**: https://huggingface.co/api
- **Implementation complexity**: Low-Medium

#### Replicate

- **Authentication**: API token
- **Metrics**: Prediction count, compute time
- **API**: https://api.replicate.com
- **Implementation complexity**: Low

#### Together AI

- **Authentication**: API key
- **Metrics**: Token usage, inference count
- **API**: Together AI API
- **Implementation complexity**: Low

## Implementation Priority

### Phase 1: Major LLM Providers (HIGH PRIORITY)

1. ✅ OpenRouter (DONE)
2. OpenAI - Most widely used
3. Anthropic Claude - Growing adoption
4. Cohere - Enterprise focus

### Phase 2: Specialized AI Assistants

1. ✅ GitHub Copilot (DONE)
2. Cursor - Popular among developers
3. Tabnine - Enterprise market
4. Codeium - Free tier popular

### Phase 3: Agent Frameworks

1. LangChain via LangSmith
2. AutoGPT
3. Crew AI
4. LangGraph

### Phase 4: Additional Platforms

1. Hugging Face
2. Replicate
3. Google AI (Gemini)
4. Mistral AI
5. Together AI

## Implementation Guide

### Adding a New Provider

See [ADDING_PROVIDERS.md](ADDING_PROVIDERS.md) for detailed instructions. Here's a quick overview:

1. **Create Provider Implementation** (`src/providers/<name>.rs`)
2. **Create Authentication Module** (`src/auth/<name>.rs`)
3. **Register Provider** (update `main.rs` and `providers/mod.rs`)
4. **Test Implementation**
5. **Update Documentation**

### Example: Adding OpenAI Support

```rust
// src/providers/openai.rs
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use super::{Provider, QuotaInfo, TokenUsage, TokenLimits};

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAICredentials {
    pub api_key: String,
}

pub struct OpenAIProvider;

#[async_trait::async_trait]
impl Provider for OpenAIProvider {
    async fn fetch_quota(&self, credentials: &str) -> Result<QuotaInfo> {
        let creds: OpenAICredentials = serde_json::from_str(credentials)?;

        let client = reqwest::Client::new();

        // Fetch usage from OpenAI API
        let response = client
            .get("https://api.openai.com/v1/usage")
            .header("Authorization", format!("Bearer {}", creds.api_key))
            .send()
            .await?;

        // Parse response and return QuotaInfo
        // ... implementation details

        Ok(QuotaInfo {
            provider: "openai".to_string(),
            account_name: "".to_string(),
            usage: TokenUsage {
                tokens_used: Some(usage_data.total_tokens),
                requests_made: Some(usage_data.total_requests),
                cost: Some(usage_data.total_cost),
            },
            limits: Some(TokenLimits {
                max_tokens: usage_data.token_limit,
                max_requests: None,
                max_cost: usage_data.billing_limit,
            }),
            reset_date: usage_data.billing_cycle_end,
            last_updated: chrono::Utc::now(),
        })
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}
```

## API Endpoints Reference

### OpenAI

- **Usage API**: `GET https://api.openai.com/v1/usage`
- **Auth**: Bearer token
- **Docs**: https://platform.openai.com/docs/api-reference

### Anthropic

- **Usage API**: `GET https://api.anthropic.com/v1/usage`
- **Auth**: x-api-key header
- **Docs**: https://docs.anthropic.com/

### Cohere

- **Usage API**: `GET https://api.cohere.ai/v1/usage`
- **Auth**: Authorization header
- **Docs**: https://docs.cohere.com/

### Hugging Face

- **Usage API**: `GET https://huggingface.co/api/usage`
- **Auth**: Bearer token
- **Docs**: https://huggingface.co/docs/api-inference

## Multi-Account Scenarios

tokstat supports multiple accounts per provider. Common scenarios:

### Development vs Production

```bash
tokstat login openai --name openai-dev
tokstat login openai --name openai-prod
```

### Team Accounts

```bash
tokstat login anthropic --name team-frontend
tokstat login anthropic --name team-backend
```

### Personal vs Work

```bash
tokstat login copilot --name personal
tokstat login copilot --name work
```

## Dashboard Display

The TUI dashboard shows:

- Provider type
- Account name
- Current usage (tokens/requests/cost)
- Limits (if available)
- Usage percentage (visual gauge)
- Last updated timestamp

## Rate Limiting Considerations

When implementing new providers:

1. Respect API rate limits
2. Cache quota data (default: 60s refresh)
3. Implement exponential backoff on errors
4. Use conditional requests (ETags) when available

## Security Best Practices

1. **Never log API keys**
2. **Store credentials in keyring** (not config files)
3. **Use HTTPS** for all API calls
4. **Validate API responses** before parsing
5. **Handle expired tokens** gracefully

## Testing New Providers

```bash
# Add provider
tokstat login <provider> --name test-account

# Test quota fetch
tokstat refresh test-account

# View in dashboard
tokstat dashboard

# Remove when done
tokstat remove test-account
```

## Contributing

To contribute a new provider:

1. Fork the repository
2. Implement the provider (see ADDING_PROVIDERS.md)
3. Add tests
4. Update this document with the new provider
5. Submit a pull request

Include in your PR:

- Provider implementation
- Authentication module
- Example usage
- API documentation links

## Community Requests

Vote for providers you want supported:

- [ ] OpenAI
- [ ] Anthropic Claude
- [ ] Cursor
- [ ] LangChain/LangSmith
- [ ] Hugging Face
- [ ] Other (open an issue!)

## Resources

- [ADDING_PROVIDERS.md](ADDING_PROVIDERS.md) - Implementation guide
- [Provider Trait Documentation](src/providers/mod.rs)
- [Authentication Modules](src/auth/)
- [Example: Copilot Provider](src/providers/copilot.rs)
- [Example: OpenRouter Provider](src/providers/openrouter.rs)

## Questions?

Open an issue on GitHub with:

- Provider name
- API documentation link
- Use case description
- Whether you can help implement it

---

**Note**: Some providers may require partnerships or special API access. We prioritize providers with public APIs that support quota/usage endpoints.
