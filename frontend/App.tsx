import { HashRouter, Routes, Route, Navigate } from 'react-router-dom'
import { HeroUIProvider } from '@heroui/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { Layout } from './components/Layout'
import { Overview } from './pages/Overview'
import { Gacha } from './pages/Gacha'
import { Playtime } from './pages/Playtime'
import { Screenshots } from './pages/Screenshots'
import { ROUTES } from './lib/constants'

const queryClient = new QueryClient()

export default function App() {
  return (
    <HeroUIProvider>
      <QueryClientProvider client={queryClient}>
        <HashRouter>
          <Layout>
            <Routes>
              <Route path={ROUTES.OVERVIEW} element={<Overview />} />
              <Route path={ROUTES.GACHA} element={<Gacha />} />
              <Route path={ROUTES.PLAYTIME} element={<Playtime />} />
              <Route path={ROUTES.SCREENSHOTS} element={<Screenshots />} />
              <Route path="*" element={<Navigate to={ROUTES.OVERVIEW} replace />} />
            </Routes>
          </Layout>
        </HashRouter>
      </QueryClientProvider>
    </HeroUIProvider>
  )
}
