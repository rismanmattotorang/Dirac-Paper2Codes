import { test, expect } from "@playwright/test"
import { mockApi } from "./fixtures"

test.beforeEach(async ({ page }) => {
  await mockApi(page)
})

test("dashboard prompts to pick a skill when none is selected", async ({ page }) => {
  await page.goto("/")
  const banner = page.getByTestId("selected-skill-banner").first()
  await expect(banner).toBeVisible()
  await expect(banner).toContainText(/No domain skill selected/i)
})

test("dashboard surfaces the selected skill before uploading", async ({ page }) => {
  // Seed a persisted selection before the app loads.
  await page.addInitScript(() => {
    window.localStorage.setItem(
      "p2c_selected_skill",
      JSON.stringify({ skillId: "quantum-computation", language: "python" }),
    )
  })
  await page.goto("/")

  const banner = page.getByTestId("selected-skill-banner").first()
  await expect(banner).toBeVisible()
  await expect(banner).toContainText("Quantum Computation")
  await expect(banner).toContainText("python")
  await expect(banner).toContainText(/guide code generation/i)
})
