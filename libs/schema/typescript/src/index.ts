export * from './schema/crafter/email_pb';
export * from './schema/devlog/devblog/entities/author_pb';
export * from './schema/devlog/devblog/entities/post_pb';
export * from './schema/devlog/devblog/rpc/post_pb';
export * from './schema/devlog/devblog/entities/interaction_pb';
export * from './schema/value/datetime_pb';
export * from './schema/value/device_pb';
export * from './schema/value/auth_method_pb';
export * from './schema/value/platform_pb';
export * from './schema/value/static_resource_pb';
export * from './schema/devlog/devblog/code-playground/rpc/code-playground_pb';
export * from './schema/devlog/devblog/code-playground/entities/file_pb';
export * from './schema/devlog/app-gateway/rpc/auth_pb'
export * from './schema/devlog/app-gateway/rpc/user_pb'
export * from './schema/devlog/app-gateway/rpc/storage_pb'
export * from './schema/devlog/app-gateway/rpc/people_pb'
export * from './schema/devlog/app-gateway/rpc/payment_pb'
export * from './schema/devlog/app-gateway/models/user_pb'
export * from './schema/devlog/app-gateway/models/device_pb'
export * from './schema/devlog/app-gateway/models/application_pb'
export * from './schema/devlog/app-gateway/models/payment_pb'

export * from './schema/midwess_ai/api/public/workspace_pb';
export * from './schema/midwess_ai/api/public/project_pb';
export * from './schema/midwess_ai/api/public/llm_provider_pb';
export * from './schema/midwess_ai/api/public/credit_pb';
export * from './schema/midwess_ai/api/internal/usage_pb';

// Midwess AI types - exported under Midwess namespace to avoid conflicts
import * as MidwessProject from './schema/midwess_ai/api/models/project_pb';
import * as MidwessTemplate from './schema/midwess_ai/api/models/models/template_pb';
import * as MidwessGitRepository from './schema/midwess_ai/api/models/models/git_repository_pb';
import * as MidwessUser from './schema/midwess_ai/api/models/user_pb';
import * as MidwessWorkspace from './schema/midwess_ai/api/models/workspace_pb';
import * as MidwessProjectRpc from './schema/midwess_ai/api/public/project_pb';
import * as MidwessCreditRpc from './schema/midwess_ai/api/public/credit_pb';
import * as MidwessUsageRpc from './schema/midwess_ai/api/internal/usage_pb';
import * as MidwessUserRpc from './schema/midwess_ai/api/public/user_pb';
import * as MidwessWorkspaceRpc from './schema/midwess_ai/api/public/workspace_pb';

export const Midwess = {
  Project: MidwessProject,
  Template: MidwessTemplate,
  GitHubRepository: MidwessGitRepository,
  User: MidwessUser,
  Workspace: MidwessWorkspace,
  ProjectRpc: MidwessProjectRpc,
  CreditRpc: MidwessCreditRpc,
  UsageRpc: MidwessUsageRpc,
  UserRpc: MidwessUserRpc,
  WorkspaceRpc: MidwessWorkspaceRpc,
};

export * as Client from './grpc/client'
