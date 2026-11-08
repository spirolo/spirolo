const importCli = (specifier: string): Promise<unknown> => import(specifier)

export const run = () => importCli('./cli.ts?case=1')
