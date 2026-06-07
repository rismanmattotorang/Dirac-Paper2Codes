import { test, expect } from "@playwright/test"
import { mockApi } from "./fixtures"

test.beforeEach(async ({ page }) => {
  await mockApi(page)
})

test("app loads with sidebar navigation", async ({ page }) => {
  await page.goto("/")
  await expect(page.getByRole("button", { name: /Domain Skills/i }).first()).toBeVisible()
  await expect(page.getByRole("button", { name: /Papers/i }).first()).toBeVisible()
  await expect(page.getByRole("button", { name: /Settings/i }).first()).toBeVisible()
})

test("navigates to the Domain Skills page", async ({ page }) => {
  await page.goto("/")
  await page.getByRole("button", { name: /Domain Skills/i }).first().click()
  await expect(page.getByRole("heading", { name: "Domain Skills" })).toBeVisible()
})
