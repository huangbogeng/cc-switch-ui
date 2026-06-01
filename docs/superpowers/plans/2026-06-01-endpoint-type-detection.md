# Endpoint Type Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a provider-form action that probes a base URL and recommends the correct `apiFormat` for Anthropic Messages, OpenAI Chat, or OpenAI Responses.

**Architecture:** Reuse the upstream `cc-switch` stream-check philosophy: probe real protocol endpoints with minimal legal request bodies, classify HTTP results, and recommend the best matching `apiFormat`. Keep probing in the backend to avoid CORS and keep the frontend limited to form UX and result display.

**Tech Stack:** Rust (`reqwest`, `axum`, existing provider helpers), React/TypeScript, Vite.

---
