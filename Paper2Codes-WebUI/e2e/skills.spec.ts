import { test, expect } from "@playwright/test"
import { mockApi } from "./fixtures"

test.beforeEach(async ({ page }) => {
  await mockApi(page)
  await page.goto("/")
  await page.getByRole("button", { name: /Domain Skills/i }).first().click()
  await expect(page.getByRole("heading", { name: "Domain Skills" })).toBeVisible()
})

test("lists the skill catalog and shows recommended libraries", async ({ page }) => {
  // Both mocked skills appear in the list.
  await expect(page.getByText("Quantum Computation").first()).toBeVisible()
  await expect(page.getByText("Computational Finance").first()).toBeVisible()

  // Selecting the quantum skill surfaces its recommended libraries.
  await page.getByRole("button", { name: /Quantum Computation/i }).first().click()
  await expect(page.getByText("qiskit")).toBeVisible()
})

test("switching the language updates the recommended libraries", async ({ page }) => {
  await page.getByRole("button", { name: /Computational Finance/i }).first().click()
  await expect(page.getByText("QuantLib")).toBeVisible()
  // Switch to C++ and verify a C++-only library shows.
  await page.getByRole("combobox").selectOption("cpp")
  await expect(page.getByText("Eigen")).toBeVisible()
})

test("selecting a skill persists the choice for the next paper", async ({ page }) => {
  await page.getByRole("button", { name: "Use skill" }).click()
  const stored = await page.evaluate(() => localStorage.getItem("p2c_selected_skill"))
  expect(stored).toContain("skillId")
})
