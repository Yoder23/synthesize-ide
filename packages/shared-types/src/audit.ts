export type AuditEvent = {
  id: string;
  sessionId: string;
  timestamp: string;
  kind:
    | 'session_started'
    | 'context_built'
    | 'model_requested'
    | 'model_response'
    | 'operation_parsed'
    | 'policy_decision'
    | 'patch_proposed'
    | 'patch_applied'
    | 'command_requested'
    | 'command_started'
    | 'command_finished'
    | 'checkpoint_created'
    | 'rollback_performed'
    | 'external_call_detected';
  payload: unknown;
};
