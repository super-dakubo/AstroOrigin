import { invoke } from '@tauri-apps/api/core';
import { useQuery, useMutation, UseQueryOptions, UseMutationOptions } from '@tanstack/react-query';

type InvokeArgs = Record<string, unknown>

export function useTauriQuery<TData>(
  command: string,
  args: InvokeArgs = {},
  options?: Omit<UseQueryOptions<TData>, 'queryKey' | 'queryFn'>
) {
  return useQuery<TData>({
    queryKey: [command, args],
    queryFn: () => invoke<TData>(command, args),
    ...options
  })
}

export function useTauriMutation<TData, TVariables = void>(
  command: string,
  options?: Omit<UseMutationOptions<TData, string, TVariables>, 'mutationFn'>
) {
  return useMutation<TData, string, TVariables>({
    mutationFn: (args) => invoke<TData>(command, args as Record<string, unknown>),
    ...options
  })
}
