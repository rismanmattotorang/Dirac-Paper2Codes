"use client"

import { IconSettings, IconLock, IconBell, IconDatabase, IconZap, IconUsers, IconSave, IconRefresh } from "@/components/ui/icons"
import { useState, useEffect } from "react"
import { formatDistanceToNow } from "date-fns"
import { settingsApi, type SettingsResponse, type UpdateSettingsRequest } from "@/lib/api/settings"
import { useToast } from "@/hooks/use-toast"
import { ApiKeysPanel } from "@/components/settings/api-keys-panel"
import { ApiTokensPanel } from "@/components/settings/api-tokens-panel"

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState("general")
  const [isSaving, setIsSaving] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const [settings, setSettings] = useState<SettingsResponse | null>(null)
  const { toast } = useToast()

  const tabs = [
    { id: "general", label: "General", icon: IconSettings },
    { id: "security", label: "Security", icon: IconLock },
    { id: "notifications", label: "Notifications", icon: IconBell },
    { id: "llm", label: "LLM Configuration", icon: IconZap },
    { id: "database", label: "Database", icon: IconDatabase },
    { id: "team", label: "Team", icon: IconUsers },
  ]

  // Load settings on mount
  useEffect(() => {
    loadSettings()
  }, [])

  const loadSettings = async () => {
    try {
      setIsLoading(true)
      const data = await settingsApi.getSettings()
      setSettings(data)
    } catch (error) {
      toast({
        title: "Error",
        description: "Failed to load settings. Please try again.",
        variant: "destructive",
      })
      console.error("Failed to load settings:", error)
    } finally {
      setIsLoading(false)
    }
  }

  const saveSettings = async () => {
    if (!settings) return

    try {
      setIsSaving(true)
      const updateRequest: UpdateSettingsRequest = {
        general: settings.general,
        notifications: settings.notifications,
        llm: {
          primary_provider: settings.llm.primary_provider,
          temperature: settings.llm.temperature,
          model_preferences: settings.llm.model_preferences,
          timeout_seconds: settings.llm.timeout_seconds,
          max_retries: settings.llm.max_retries,
        },
        database: {
          connection_string: settings.database.connection_string,
          namespace: settings.database.namespace,
          database: settings.database.database,
          max_connections: settings.database.max_connections,
          backup_schedule: settings.database.backup_schedule,
        },
      }
      
      const updatedSettings = await settingsApi.updateSettings(updateRequest)
      setSettings(updatedSettings)
      
      toast({
        title: "Success",
        description: "Settings saved successfully",
      })
    } catch (error) {
      toast({
        title: "Error",
        description: "Failed to save settings. Please try again.",
        variant: "destructive",
      })
      console.error("Failed to save settings:", error)
    } finally {
      setIsSaving(false)
    }
  }

  if (isLoading) {
    return (
      <div className="p-6 flex items-center justify-center min-h-screen">
        <div className="flex items-center gap-2">
          <IconRefresh className="w-5 h-5 animate-spin" />
          <span>Loading settings...</span>
        </div>
      </div>
    )
  }

  if (!settings) {
    return (
      <div className="p-6">
        <div className="p-4 rounded-lg bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800">
          <p className="text-sm font-medium text-red-900 dark:text-red-300">
            Failed to load settings. Please refresh the page.
          </p>
        </div>
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-neutral-900 dark:text-white">Settings</h1>
        <p className="text-neutral-600 dark:text-neutral-400 mt-1">Manage your Paper2Codes configuration</p>
      </div>

      {/* Tab Navigation */}
      <div className="card-base overflow-hidden">
        <div className="border-b border-neutral-200 dark:border-neutral-700">
          <div className="flex overflow-x-auto">
            {tabs.map((tab) => {
              const Icon = tab.icon
              const isActive = activeTab === tab.id
              return (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={`flex items-center gap-2 px-4 py-3 font-semibold text-sm whitespace-nowrap transition-colors ${
                    isActive
                      ? "border-b-2 border-blue-600 text-blue-600 dark:text-blue-400"
                      : "text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white"
                  }`}
                >
                  <Icon className="w-4 h-4" />
                  {tab.label}
                </button>
              )
            })}
          </div>
        </div>

        {/* Tab Content */}
        <div className="p-6 space-y-6">
          {activeTab === "general" && <GeneralSettings settings={settings} setSettings={setSettings} />}
          {activeTab === "security" && <SecuritySettings settings={settings} />}
          {activeTab === "notifications" && <NotificationSettings settings={settings} setSettings={setSettings} />}
          {activeTab === "llm" && <LLMSettings settings={settings} setSettings={setSettings} />}
          {activeTab === "database" && <DatabaseSettings settings={settings} setSettings={setSettings} />}
          {activeTab === "team" && <TeamSettings settings={settings} />}

          {/* Save Button */}
          <div className="flex justify-end pt-4 border-t border-neutral-200 dark:border-neutral-700">
            <button
              onClick={saveSettings}
              className="btn-primary flex items-center gap-2"
              disabled={isSaving}
            >
              <IconSave className="w-4 h-4" />
              {isSaving ? "Saving..." : "Save Changes"}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

function GeneralSettings({ settings, setSettings }: { 
  settings: SettingsResponse; 
  setSettings: React.Dispatch<React.SetStateAction<SettingsResponse | null>> 
}) {
  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-semibold text-neutral-900 dark:text-white mb-2">Organization Name</label>
          <input
            type="text"
            placeholder="Your Organization"
            value={settings.general.organization_name}
            onChange={(e) => setSettings(prev => prev ? {
              ...prev,
              general: { ...prev.general, organization_name: e.target.value }
            } : null)}
            className="input-base w-full"
          />
        </div>
        <div>
          <label className="block text-sm font-semibold text-neutral-900 dark:text-white mb-2">
            Default Paper Processing Domain
          </label>
          <select 
            className="input-base w-full"
            value={settings.general.default_domain}
            onChange={(e) => setSettings(prev => prev ? {
              ...prev,
              general: { ...prev.general, default_domain: e.target.value }
            } : null)}
          >
            <option>Auto-detect</option>
            <option>Deep Learning</option>
            <option>NLP</option>
            <option>Computational Physics</option>
          </select>
        </div>
      </div>

      <div className="flex items-center justify-between p-4 rounded-lg bg-neutral-50 dark:bg-neutral-700/50">
        <div>
          <p className="font-semibold text-neutral-900 dark:text-white">Dark Mode</p>
          <p className="text-sm text-neutral-600 dark:text-neutral-400">Enable dark theme for all interfaces</p>
        </div>
        <input 
          type="checkbox" 
          className="w-5 h-5 rounded" 
          checked={settings.general.dark_mode}
          onChange={(e) => setSettings(prev => prev ? {
            ...prev,
            general: { ...prev.general, dark_mode: e.target.checked }
          } : null)}
        />
      </div>
    </div>
  )
}

function SecuritySettings({ settings }: { settings: SettingsResponse }) {
  const { toast } = useToast()
  
  const handleEnable2FA = async () => {
    try {
      const response = await settingsApi.enable2FA()
      toast({
        title: "2FA Enabled",
        description: "Scan the QR code with your authenticator app",
      })
      // TODO: Show QR code modal
      console.log("2FA Setup:", response)
    } catch (error) {
      toast({
        title: "Error",
        description: "Failed to enable 2FA",
        variant: "destructive",
      })
    }
  }

  const handleRevokeSession = async (sessionId: string) => {
    try {
      await settingsApi.revokeSession(sessionId)
      toast({
        title: "Success",
        description: "Session revoked successfully",
      })
      // Reload settings
      window.location.reload()
    } catch (error) {
      toast({
        title: "Error",
        description: "Failed to revoke session",
        variant: "destructive",
      })
    }
  }

  return (
    <div className="space-y-6">
      <div className={`p-4 rounded-lg border ${
        settings.security.two_factor_enabled 
          ? "bg-green-50 dark:bg-green-950/30 border-green-200 dark:border-green-800"
          : "bg-blue-50 dark:bg-blue-950/30 border-blue-200 dark:border-blue-800"
      }`}>
        <p className={`text-sm font-medium ${
          settings.security.two_factor_enabled
            ? "text-green-900 dark:text-green-300"
            : "text-blue-900 dark:text-blue-300"
        }`}>
          {settings.security.two_factor_enabled 
            ? "Two-factor authentication is enabled" 
            : "Two-factor authentication is currently disabled"}
        </p>
        <button 
          className="mt-3 btn-primary text-sm"
          onClick={handleEnable2FA}
        >
          {settings.security.two_factor_enabled ? "Reconfigure 2FA" : "Enable 2FA"}
        </button>
      </div>

      <div>
        <h3 className="font-semibold text-neutral-900 dark:text-white mb-3">Active Sessions</h3>
        <div className="space-y-3">
          {settings.security.active_sessions.map((session) => (
            <div
              key={session.id}
              className="flex items-center justify-between p-3 rounded-lg bg-neutral-100 dark:bg-neutral-700/50"
            >
              <div>
                <p className="font-medium text-neutral-900 dark:text-white">{session.device}</p>
                <p className="text-xs text-neutral-500 dark:text-neutral-400">{session.ip}</p>
              </div>
              <div className="text-right">
                <p className="text-xs text-neutral-600 dark:text-neutral-400">{session.last_active}</p>
                <button 
                  className="text-xs text-red-600 dark:text-red-400 hover:underline"
                  onClick={() => handleRevokeSession(session.id)}
                >
                  Revoke
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>

      <ApiTokensPanel />
    </div>
  )
}

function NotificationSettings({ settings, setSettings }: {
  settings: SettingsResponse; 
  setSettings: React.Dispatch<React.SetStateAction<SettingsResponse | null>> 
}) {
  const notifications = [
    { key: "paper_processing_complete", name: "Paper Processing Complete" },
    { key: "code_generation_errors", name: "Code Generation Errors" },
    { key: "task_queue_updates", name: "Task Queue Updates" },
    { key: "weekly_report", name: "Weekly Report" },
  ] as const

  return (
    <div className="space-y-4">
      {notifications.map((notif) => (
        <div
          key={notif.key}
          className="flex items-center justify-between p-4 rounded-lg bg-neutral-50 dark:bg-neutral-700/50"
        >
          <p className="font-medium text-neutral-900 dark:text-white">{notif.name}</p>
          <input 
            type="checkbox" 
            className="w-5 h-5 rounded" 
            checked={settings.notifications[notif.key]}
            onChange={(e) => setSettings(prev => prev ? {
              ...prev,
              notifications: { ...prev.notifications, [notif.key]: e.target.checked }
            } : null)}
          />
        </div>
      ))}
    </div>
  )
}

function LLMSettings({ settings, setSettings }: {
  settings: SettingsResponse;
  setSettings: React.Dispatch<React.SetStateAction<SettingsResponse | null>>
}) {
  return (
    <div className="space-y-6">
      {/* Provider keys are managed live (set/test/remove/default) by this panel
          and take effect immediately, independent of the Save Changes button. */}
      <ApiKeysPanel />

      <div className="border-t border-neutral-200 dark:border-neutral-700 pt-6 grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-semibold text-neutral-900 dark:text-white mb-2">
            Temperature: {settings.llm.temperature}
          </label>
          <input 
            type="range" 
            min="0" 
            max="2" 
            step="0.1" 
            value={settings.llm.temperature}
            onChange={(e) => setSettings(prev => prev ? {
              ...prev,
              llm: { ...prev.llm, temperature: parseFloat(e.target.value) }
            } : null)}
            className="w-full" 
          />
        </div>

      <div>
        <label className="block text-sm font-semibold text-neutral-900 dark:text-white mb-2">Model Preferences</label>
        <div className="space-y-3">
          {[
            { task: "Planning", key: "planning" },
            { task: "Analysis", key: "analysis" },
            { task: "Coding", key: "coding" },
            { task: "Verification", key: "verification" },
          ].map((pref) => (
            <div key={pref.key} className="grid grid-cols-2 gap-2 items-center">
              <label className="text-sm text-neutral-600 dark:text-neutral-400">{pref.task}:</label>
              <select 
                className="input-base text-sm"
                value={settings.llm.model_preferences[pref.key as keyof typeof settings.llm.model_preferences]}
                onChange={(e) => setSettings(prev => prev ? {
                  ...prev,
                  llm: { 
                    ...prev.llm, 
                    model_preferences: { 
                      ...prev.llm.model_preferences, 
                      [pref.key]: e.target.value 
                    } 
                  }
                } : null)}
              >
                <option value="openai/gpt-4-turbo">GPT-4 Turbo</option>
                <option value="anthropic/claude-3-opus">Claude 3 Opus</option>
                <option value="x-ai/grok-2">Grok-2</option>
              </select>
            </div>
          ))}
        </div>
      </div>
      </div>
    </div>
  )
}

function DatabaseSettings({ settings, setSettings }: { 
  settings: SettingsResponse; 
  setSettings: React.Dispatch<React.SetStateAction<SettingsResponse | null>> 
}) {
  const { toast } = useToast()
  const [isTesting, setIsTesting] = useState(false)

  const handleTestConnection = async () => {
    try {
      setIsTesting(true)
      const result = await settingsApi.testDatabaseConnection()
      setSettings(prev => prev ? {
        ...prev,
        database: {
          ...prev.database,
          connected: result.success,
          schema_valid: result.schema_valid ?? result.success,
          last_sync: result.success ? new Date().toISOString() : prev.database.last_sync,
        }
      } : null)
      toast({
        title: result.success ? "Success" : "Failed",
        description: result.message,
        variant: result.success ? "default" : "destructive",
      })
    } catch (error: any) {
      console.error("Database connection test error:", error)
      const errorMessage = error?.message || error?.error?.message || "Failed to test database connection"
      toast({
        title: "Error",
        description: errorMessage,
        variant: "destructive",
      })
    } finally {
      setIsTesting(false)
    }
  }

  const isConnected = settings.database.connected
  const schemaValid = settings.database.schema_valid
  const lastSyncDisplay = (() => {
    const lastSync = settings.database.last_sync
    if (!lastSync) return null
    const parsed = new Date(lastSync)
    if (Number.isNaN(parsed.getTime())) {
      return lastSync
    }
    return formatDistanceToNow(parsed, { addSuffix: true })
  })()

  const statusStyles = isConnected
    ? schemaValid
      ? "bg-green-50 dark:bg-green-950/30 border-green-200 dark:border-green-800"
      : "bg-amber-50 dark:bg-amber-950/30 border-amber-200 dark:border-amber-800"
    : "bg-red-50 dark:bg-red-950/30 border-red-200 dark:border-red-800"

  const statusText = isConnected
    ? schemaValid
      ? "✓ Connected to SurrealDB"
      : "⚠ Connected, schema validation required"
    : "✗ Not Connected"

  return (
    <div className="space-y-6">
      <div className={`p-4 rounded-lg border ${statusStyles}`}>
        <p className={`text-sm font-semibold ${
          isConnected
            ? schemaValid
              ? "text-green-900 dark:text-green-300"
              : "text-amber-900 dark:text-amber-300"
            : "text-red-900 dark:text-red-300"
        }`}>
          {statusText}
        </p>
        {isConnected && (
          <p className="text-xs text-muted-foreground mt-1">
            Namespace: <span className="font-medium">{settings.database.namespace}</span> • Database:{" "}
            <span className="font-medium">{settings.database.database}</span>
            {lastSyncDisplay && (
              <> • Last sync: {lastSyncDisplay}</>
            )}
            {!schemaValid && (
              <> • Schema validation required (enable auto migrate or run migrations)</>
            )}
          </p>
        )}
        <button 
          className="mt-3 btn-primary text-sm flex items-center gap-2"
          onClick={handleTestConnection}
          disabled={isTesting}
        >
          <IconRefresh className={`w-3 h-3 ${isTesting ? 'animate-spin' : ''}`} />
          {isTesting ? "Testing..." : "Test Connection"}
        </button>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-semibold text-neutral-900 dark:text-white mb-2">Database Host</label>
          <input 
            type="text" 
            value={settings.database.connection_string}
            onChange={(e) => setSettings(prev => prev ? {
              ...prev,
              database: { ...prev.database, connection_string: e.target.value, connected: false, schema_valid: false }
            } : null)}
            className="input-base w-full" 
          />
        </div>
        <div>
          <label className="block text-sm font-semibold text-neutral-900 dark:text-white mb-2">Namespace</label>
          <input 
            type="text" 
            value={settings.database.namespace}
            onChange={(e) => setSettings(prev => prev ? {
              ...prev,
              database: { ...prev.database, namespace: e.target.value, schema_valid: false }
            } : null)}
            className="input-base w-full" 
          />
        </div>
      </div>

      <div>
        <label className="block text-sm font-semibold text-neutral-900 dark:text-white mb-2">Backup Schedule</label>
        <select 
          className="input-base w-full"
          value={settings.database.backup_schedule}
          onChange={(e) => setSettings(prev => prev ? {
            ...prev,
            database: { ...prev.database, backup_schedule: e.target.value }
          } : null)}
        >
          <option>Daily at 2:00 AM UTC</option>
          <option>Weekly</option>
          <option>Manual Only</option>
        </select>
      </div>
    </div>
  )
}

function TeamSettings({ settings }: { settings: SettingsResponse }) {
  const { toast } = useToast()
  const [isAddingMember, setIsAddingMember] = useState(false)

  const handleAddMember = async () => {
    // TODO: Show add member modal
    setIsAddingMember(true)
    toast({
      title: "Coming Soon",
      description: "Team member management will be available soon",
    })
    setIsAddingMember(false)
  }

  const handleRemoveMember = async (memberId: string) => {
    try {
      await settingsApi.removeTeamMember(memberId)
      toast({
        title: "Success",
        description: "Team member removed successfully",
      })
      window.location.reload()
    } catch (error) {
      toast({
        title: "Error",
        description: "Failed to remove team member",
        variant: "destructive",
      })
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h3 className="font-semibold text-neutral-900 dark:text-white">Team Members</h3>
        <button 
          className="btn-primary text-sm"
          onClick={handleAddMember}
          disabled={isAddingMember}
        >
          Add Member
        </button>
      </div>

      <div className="space-y-3">
        {settings.team.members.map((member) => (
          <div
            key={member.id}
            className="flex items-center justify-between p-4 rounded-lg bg-neutral-50 dark:bg-neutral-700/50"
          >
            <div>
              <p className="font-medium text-neutral-900 dark:text-white">{member.name}</p>
              <p className="text-xs text-neutral-500 dark:text-neutral-400">{member.email}</p>
            </div>
            <div className="text-right flex-shrink-0">
              <p className="text-xs font-semibold text-neutral-600 dark:text-neutral-400">{member.role}</p>
              <p
                className={`text-xs ${member.status === "Active" ? "text-green-600 dark:text-green-400" : "text-amber-600 dark:text-amber-400"}`}
              >
                {member.status}
              </p>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
