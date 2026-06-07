"use client"

import { Header } from "@/components/layout/header"
import { Sidebar } from "@/components/layout/sidebar"
import { AppRouter, useRouter, PageRenderer } from "@/components/app-router"
import { useState } from "react"

function PageContent() {
  const { currentPage } = useRouter()
  const [sidebarOpen, setSidebarOpen] = useState(true)

  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <Sidebar open={sidebarOpen} onToggle={setSidebarOpen} />
      <div className="flex-1 flex flex-col overflow-hidden min-w-0">
        <Header onSidebarToggle={() => setSidebarOpen(!sidebarOpen)} />
        <main
          id="main-content"
          className="flex-1 overflow-auto focus:outline-none"
          role="main"
          aria-label="Main content"
          tabIndex={-1}
        >
          <PageRenderer page={currentPage} />
        </main>
      </div>
    </div>
  )
}

export default function Home() {
  return (
    <AppRouter>
      <PageContent />
    </AppRouter>
  )
}
