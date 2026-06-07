export function LoadingSkeleton() {
  return (
    <div className="space-y-4">
      {[...Array(3)].map((_, i) => (
        <div key={i} className="space-y-2">
          <div className="h-4 bg-muted rounded w-3/4 animate-pulse"></div>
          <div className="h-3 bg-muted rounded w-1/2 animate-pulse"></div>
        </div>
      ))}
    </div>
  )
}
