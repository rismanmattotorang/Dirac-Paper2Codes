import { test, expect, type Page } from "@playwright/test"
import { mockApi } from "./fixtures"

const MOCK_USER = {
  id: "u1",
  email: "alice@dirac.id",
  username: "alice",
  roles: ["admin"],
}

const LOGIN_PAYLOAD = {
  success: true,
  data: {
    access_token: "access-token-abc",
    refresh_token: "refresh-token-xyz",
    token_type: "Bearer",
    expires_in: 900,
    user: MOCK_USER,
  },
}

async function mockAuth(page: Page) {
  await page.route("**/api/auth/login", async (route) => {
    await route.fulfill({ json: LOGIN_PAYLOAD })
  })
  await page.route("**/api/auth/me", async (route) => {
    await route.fulfill({ json: { success: true, data: MOCK_USER } })
  })
}

test.beforeEach(async ({ page }) => {
  await mockApi(page)
})

test("anonymous users see a Sign in affordance, not a user menu", async ({ page }) => {
  await page.goto("/")
  await expect(page.getByTestId("sign-in-button")).toBeVisible()
  await expect(page.getByTestId("user-menu-trigger")).toHaveCount(0)
})

test("a user can sign in and then sees their account menu", async ({ page }) => {
  await mockAuth(page)
  await page.goto("/")

  await page.getByTestId("sign-in-button").click()
  await expect(page.getByTestId("login-dialog")).toBeVisible()

  await page.locator("#login-email").fill("alice@dirac.id")
  await page.locator("#login-password").fill("password123")
  await page.getByTestId("login-submit").click()

  // The header swaps the Sign-in button for the user avatar/menu.
  await expect(page.getByTestId("user-menu-trigger")).toBeVisible({ timeout: 15000 })
  await expect(page.getByTestId("sign-in-button")).toHaveCount(0)

  // The access token is persisted for subsequent API calls.
  const token = await page.evaluate(() => localStorage.getItem("auth_token"))
  expect(token).toBe("access-token-abc")
})
