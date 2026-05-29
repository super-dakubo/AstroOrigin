import { create } from 'zustand';

interface GachaState {
  sortOrder: 'desc' | 'asc';
  setSortOrder: (order: 'desc' | 'asc') => void;
  filterStar: number | null;
  setFilterStar: (star: number | null) => void;
}

export const useGachaStore = create<GachaState>((set) => ({
  sortOrder: 'desc',
  setSortOrder: (sortOrder) => set({ sortOrder }),
  filterStar: null,
  setFilterStar: (filterStar) => set({ filterStar }),
}));
