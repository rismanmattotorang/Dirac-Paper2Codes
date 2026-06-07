import { test, expect, type Page } from "@playwright/test"
import { mockApi, mockSettingsApi } from "./fixtures"

test.beforeEach(async ({ page }) => {
  await mockApi(page)
  await mockSettingsApi(page)
})

async function openLlmTab(page: Page) {
  await page.goto("/")
  await page.getByRole("button", { name: /Settings/i }).first().click()
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible()
  await page.getByRole("button", { name: /LLM Configuration/i }).click()
  await expect(page.getByText("Provider API Keys")).toBeVisible()
}

test("settings page renders its tabs", async ({ page }) => {
  await page.goto("/")
  await page.getByRole("button", { name: /Settings/i }).first().click()
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible()
  await expect(page.getByRole("button", { name: /LLM Configuration/i })).toBeVisible()
  await expect(page.getByRole("button", { name: /Database/i })).toBeVisible()
})

test("LLM tab lists provider keys with their status", async ({ page }) => {
  await openLlmTab(page)
  await expect(page.getByTestId("provider-openai")).toBeVisible()
  // The default provider is badged, and a configured key shows masked.
  await expect(page.getByTestId("provider-openai").getByText("Default")).toBeVisible()
  await expect(page.getByTestId("provider-openai").getByText("sk-p…wxyz")).toBeVisible()
  // An unconfigured provider is clearly marked.
  await expect(page.getByTestId("provider-anthropic").getByText("Not configured")).toBeVisible()
})
