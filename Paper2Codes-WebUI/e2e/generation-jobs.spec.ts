import { test, expect, type Page } from "@playwright/test"
import { mockApi } from "./fixtures"

const MOCK_JOBS = [
  {
    id: "11111111-1111-1111-1111-111111111111",
    kind: "paper_generation",
    paper_id: "paper-abc",
    status: "running",
    progress: 0.4,
    attempts: 1,
    max_attempts: 2,
    last_error: null,
    created_at: "2026-01-02T00:00:00Z",
    updated_at: "2026-01-02T00:01:00Z",
  },
  {
    id: "22222222-2222-2222-2222-222222222222",
    kind: "paper_generation",
    paper_id: "paper-def",
    status: "completed",
    progress: 1.0,
    attempts: 1,
    max_attempts: 2,
    last_error: null,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:05:00Z",
  },
]

async function mockJobs(page: Page) {
  await page.route("**/api/jobs*", async (route) => {
    await route.fulfill({ json: { success: true, data: MOCK_JOBS } })
  })
}

async function openGeneratedCode(page: Page) {
  await page.goto("/")
  await page.getByRole("button", { name: /Generated Code/i }).first().click()
  await expect(page.getByTestId("generation-jobs-panel")).toBeVisible()
}

test.beforeEach(async ({ page }) => {
  await mockApi(page)
  await mockJobs(page)
})

test("the Generated Code page lists durable generation runs", async ({ page }) => {
  await openGeneratedCode(page)

  const jobs = page.getByTestId("generation-job")
  await expect(jobs).toHaveCount(2)

  // Running job shows its paper + a cancel affordance; completed does not.
  await expect(page.getByText("Paper paper-abc")).toBeVisible()
  await expect(page.getByText("running")).toBeVisible()
  await expect(page.getByText("completed")).toBeVisible()
  await expect(page.getByRole("button", { name: /Cancel/i })).toHaveCount(1)
})
