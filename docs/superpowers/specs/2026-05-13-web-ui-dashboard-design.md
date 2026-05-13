# Tantivy CLI Web UI — Dashboard First Design

## Overview

Full overhaul of the existing Tantivy CLI web frontend. The current UI is a functional prototype with two views (HomeView, IndexView). The redesign transforms it into a team dashboard for managing search infrastructure, with a collapsible sidebar, multi-page layout, dark/light theme support, and four major feature areas: Dashboard, Advanced Search, Document Management, and Monitoring & Maintenance.

**Stack:** Vue 3 + Vue Router + Headless UI + Heroicons + Tailwind CSS + Vite + Axios (existing dependencies, no new packages needed)

**Target users:** Team managing search infrastructure, needs monitoring dashboards, health checks, multi-index overview.

## Navigation & Layout

### Top Bar
- Fixed height (48px)
- Left: Logo + hamburger sidebar toggle + breadcrumb
- Right: Theme toggle (Headless UI `Switch`) + server status indicator

### Collapsible Sidebar
- Icon-only mode (collapsed) or icon + label (expanded)
- 4 items:
  1. Dashboard (`ChartBarIcon`) → `/`
  2. Indexes (`FolderIcon`) → `/indexes`
  3. Search (`MagnifyingGlassIcon`) → `/search`
  4. Settings (`CogIcon`) → `/settings`
- On mobile: sidebar becomes a drawer overlay triggered by hamburger menu (Headless UI `Dialog`)

### Theme System
- Tailwind `darkMode: 'class'` strategy
- Toggle `dark` class on `<html>` element
- Persist preference in `localStorage`
- Three modes: Light, Dark, System (follows `prefers-color-scheme`)

### Routes
```
/                    → Dashboard
/indexes             → Index list + management
/index/:name         → Index detail (tabs: Overview, Search, Documents, Operations)
/search              → Global search across all indexes
/settings            → Theme toggle, server info
```

## Dashboard Page (`/`)

### Summary Cards
- 4 cards in responsive grid (`grid-cols-2 md:grid-cols-4`)
- Indexes (total count), Documents (sum of all `num_docs`), Segments (sum of all `num_segments`), Status ("Healthy" or "Needs Attention")
- Each card: icon, large number, label

### Index Table
- Columns: Name (clickable), Documents, Segments, Size, Status
- Status indicators: green dot (active), yellow dot (dirty/needs commit), gray dot (idle)
- Row hover reveals quick-action buttons (Search, Delete)
- `+ New Index` button opens creation dialog (Headless UI `Dialog` with name + schema JSON fields)

### Auto-Refresh
- Polls `GET /indexes` + parallel `GET /indexes/:name/stats` for each index
- 30-second interval
- Displays "Last updated: HH:MM" timestamp
- Pause button to stop auto-refresh during manual work
- Uses `useAutoRefresh` composable

### API
- `GET /indexes` → index list
- Fan-out parallel `GET /indexes/:name/stats` for each index
- No new API endpoints needed

## Index Detail Page (`/index/:name`)

### Header Area
- Back button → `/indexes`
- Index name as page title
- 3 stat cards: Documents, Segments, Size (from stats)
- Action buttons: Rebuild, Compress, Delete (red, with confirmation dialog)

### Tab 1 — Overview
- Schema table: columns for Field name, Type, Stored, Indexed, Fast — parsed from `stats.schema`
- Segment details table if available from stats

### Tab 2 — Search
- Search input with Enter-to-submit + limit dropdown (10/20/50/100)
- Results display parsed document fields (not raw JSON by default)
- Highlight matches rendered as `<mark>` with yellow background
- "X results (time)" header above result list
- Collapsible "Raw JSON" panel at bottom (Headless UI `Disclosure`) for debugging

### Tab 3 — Documents
- Table view auto-generates columns from schema's stored fields
- Pagination with limit/offset + "Showing X-Y of Z"
- "Add Document" button opens dialog with JSON textarea
- "Bulk Import" button opens dialog with textarea for multiple JSON docs (one per line, calls `_bulk` endpoint)
- Row click opens document detail in a slide-over panel (Headless UI `Dialog` positioned as side panel)

### Tab 4 — Operations
- Maintenance section: Rebuild Index + Compress Index buttons
- Danger Zone section (red border): Delete Index with "This action cannot be undone" warning
- Index Info section: path, created date, modified date if available

## Global Search Page (`/search`)

- Large centered search bar (hero-style)
- Index filter (Headless UI `Listbox`): "All Indexes" or pick a specific index
- Limit selector
- Results grouped by index name with collapsible sections
- Each result card links to the corresponding index detail page
- API: for "All Indexes", iterates all indexes and calls `GET /indexes/:name/search` for each

## Settings Page (`/settings`)

- Appearance section: Theme selector using Headless UI `RadioGroup` — Light / Dark / System
- Server section: URL, version, uptime display

## Component Architecture

### File Structure
```
web/src/
├── main.js                    # App entry, router, global plugins
├── App.vue                    # Top bar + sidebar layout
├── api.js                     # Axios instance + all API functions
├── composables/
│   ├── useTheme.js            # Theme toggle (localStorage + class toggle)
│   └── useAutoRefresh.js      # Polling logic with pause/resume
├── components/
│   ├── AppSidebar.vue         # Collapsible sidebar
│   ├── AppTopbar.vue          # Top bar with breadcrumb + theme switch
│   ├── StatCard.vue           # Reusable stat card
│   ├── IndexTable.vue         # Index list table with status indicators
│   ├── SearchBar.vue          # Search input + limit + submit
│   ├── SearchResult.vue       # Single search result card
│   ├── DocTable.vue           # Document table with auto-columns
│   ├── DocDetail.vue          # Document detail slide-over
│   ├── SchemaTable.vue        # Schema field table
│   ├── ConfirmDialog.vue      # Reusable confirmation dialog
│   ├── ThemeToggle.vue        # Theme switch component
│   └── Pagination.vue         # Page navigation
├── views/
│   ├── DashboardView.vue      # Dashboard overview
│   ├── IndexListView.vue      # Index management
│   ├── IndexDetailView.vue    # Index detail with tabs
│   ├── GlobalSearchView.vue   # Cross-index search
│   └── SettingsView.vue       # Theme + server settings
└── style.css                  # Tailwind directives + dark mode base
```

### Data Flow
- **Views** own state and call API functions. Pass data down as props.
- **Components** are pure presentational — receive props, emit events.
- **Composables** encapsulate shared reactive logic.

### Headless UI Component Mapping

| Component          | Headless UI Primitive          |
|--------------------|--------------------------------|
| Sidebar (mobile)   | `Dialog` + `TransitionRoot`    |
| Tabs (index detail)| `TabGroup`, `TabList`, `Tab`   |
| Create dialog      | `Dialog` + `TransitionRoot`    |
| Confirm delete     | `Dialog`                       |
| Doc detail panel   | `Dialog` (positioned as panel) |
| Index filter       | `Listbox`                      |
| Theme selector     | `RadioGroup`                   |
| Theme toggle (top) | `Switch`                       |
| Collapsible JSON   | `Disclosure`                   |

## API Usage

All existing API endpoints are used. No new backend endpoints required:

- `GET /indexes` — list all indexes
- `POST /indexes/:name` — create index with schema
- `DELETE /indexes/:name` — delete index
- `GET /indexes/:name/stats` — index statistics
- `GET /indexes/:name/search` — search with query
- `POST /indexes/:name/docs` — add single document
- `POST /indexes/:name/docs/_bulk` — bulk add documents
- `GET /indexes/:name/docs` — list documents with pagination
- `POST /indexes/:name/rebuild` — trigger rebuild
- `POST /indexes/:name/compress` — trigger compress

## Key Design Decisions

1. **Dashboard as landing page** — teams need overview first, not a blank state
2. **No new dependencies** — Vue 3 + Headless UI + Tailwind already installed
3. **Composables for shared logic** — theme and auto-refresh are cross-cutting concerns
4. **Components are presentational** — keeps views as the single source of truth for page state
5. **Parallel stat fetching** — fan-out requests for dashboard rather than a single aggregate endpoint
6. **SPA fallback** — existing `rust-embed` + SPA fallback in `ui.rs` handles all client-side routes
