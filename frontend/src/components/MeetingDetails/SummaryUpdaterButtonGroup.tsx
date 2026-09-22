"use client";

import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, Save, Loader2, Send } from 'lucide-react';
import Analytics from '@/lib/analytics';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { useState } from 'react';

interface SummaryUpdaterButtonGroupProps {
  isSaving: boolean;
  isDirty: boolean;
  meetingId: string;
  onSave: () => Promise<void>;
  onCopy: () => Promise<void>;
}

export function SummaryUpdaterButtonGroup({
  isSaving,
  isDirty,
  meetingId,
  onSave,
  onCopy,
}: SummaryUpdaterButtonGroupProps) {
  const [publishing, setPublishing] = useState(false);

  const handlePublish = async () => {
    setPublishing(true);
    try {
      const result = await invoke<{ id?: string }>('citywalk_publish_meeting', { meetingId });
      toast.success(result?.id ? `Sent to CityWalk (${result.id})` : 'Sent to CityWalk');
    } catch (error) {
      toast.error(String(error) || 'Could not send this meeting to CityWalk');
    } finally {
      setPublishing(false);
    }
  };

  return (
    <ButtonGroup>
      {/* Save button */}
      <Button
        variant="outline"
        size="sm"
        className={`${isDirty ? 'bg-green-200' : ""}`}
        title={isSaving ? "Saving" : "Save Changes"}
        onClick={() => {
          Analytics.trackButtonClick('save_changes', 'meeting_details');
          onSave();
        }}
        disabled={isSaving}
      >
        {isSaving ? (
          <>
            <Loader2 className="animate-spin" />
            <span className="hidden @[40rem]:inline">Saving...</span>
          </>
        ) : (
          <>
            <Save />
            <span className="hidden @[40rem]:inline">Save</span>
          </>
        )}
      </Button>

      {/* Copy button */}
      <Button
        variant="outline"
        size="sm"
        title="Copy Summary"
        onClick={() => {
          Analytics.trackButtonClick('copy_summary', 'meeting_details');
          onCopy();
        }}
        className="cursor-pointer"
      >
        <Copy />
        <span className="hidden @[40rem]:inline">Copy</span>
      </Button>

      <Button
        variant="outline"
        size="sm"
        title="Send minutes to CityWalk Task Manager"
        disabled={publishing}
        onClick={() => {
          Analytics.trackButtonClick('send_to_citywalk', 'meeting_details');
          void handlePublish();
        }}
      >
        {publishing ? <Loader2 className="animate-spin" /> : <Send />}
        <span className="hidden @[40rem]:inline">{publishing ? 'Sending...' : 'CityWalk'}</span>
      </Button>

    </ButtonGroup>
  );
}
