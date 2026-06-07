import { test, expect, type Page } from "@playwright/test"
import { mockApi, mockSettingsApi } from "./fixtures"

test.beforeEach(async ({ page }) => {
  await mockApi(page)
  await mockSettingsApi(page)
})

async function openLlmTab(page: Page) {
  await page.goto("/")
  await page.getByRole("button", { name: /Settings/i }).first().click()
  await page.getByRole("button", { name: /LLM Configuration/i }).click()
  await expect(page.getByText("Provider API Keys")).toBeVisible()
}

test("validates a configured key via the Test button", async ({ page }) => {
  await openLlmTab(page)
  const card = page.getByTestId("provider-openai")
  await card.getByRole("button", { name: "Test" }).click()
  await expect(card.getByText(/valid/i)).toBeVisible()
})

test("sets a configured provider as the default", async ({ page }) => {
  await openLlmTab(page)
  // OpenRouter is configured but not default → it offers "Set default".
  // Match the badge exactly so it doesn't also match the "Set default" button.
  const card = page.getByTestId("provider-openrouter")
  await expect(card.getByText("Default", { exact: true })).toHaveCount(0)
  await card.getByRole("button", { name: /Set default/i }).click()
  await expect(card.getByText("Default", { exact: true })).toBeVisible()
})

test("saves a key for an unconfigured provider", async ({ page }) => {
  await openLlmTab(page)
  const card = page.getByTestId("provider-anthropic")
  await expect(card.getByText("Not configured")).toBeVisible()
  await card.locator('input[type="password"]').fill("sk-ant-test-key-1234567890")
  await card.getByRole("button", { name: /^(Save|Update)$/ }).click()
  // After saving, the provider reports as configured.
  await expect(card.getByText("Configured")).toBeVisible()
})
