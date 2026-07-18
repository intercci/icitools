#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.13"
# dependencies = [
#   "boto3==1.37.22"
# ]
# ///

import json
import sys
import boto3
import uuid

# filter for sids of duplicate policies
def filter_policies(fn_name):
  client = boto3.client('lambda') 
  policy = client.get_policy(FunctionName=fn_name)['Policy']
  statements = json.loads(policy)['Statement'] 
  unique_policies = []
  duplicate_sids = []
  for item in statements:
    arn = item['Condition']['ArnLike']['AWS:SourceArn']
    if (arn not in unique_policies):
      unique_policies.append(arn)
    else:
      duplicate_sids.append(item['Sid'])
  
  return unique_policies, duplicate_sids

# remove duplicate policies
def clean_up_policies(sids, fn_name):
  client = boto3.client('lambda')

  for sid in sids:       
    # print('remove SID {}'.format(sid))
    client.remove_permission(
      FunctionName=fn_name, 
      StatementId=sid
    )

def clone_policy(policy):
    p = policy
    p['Sid'] = str(uuid.uuid4())
    sarn = p['Condition']['ArnLike']['AWS:SourceArn']
    p['Condition']['ArnLike']['AWS:SourceArn'] = sarn[:sarn.find('/*')+2]
    return p

def clean_policy(fn_name):
    client = boto3.client('lambda') 
    policy = client.get_policy(FunctionName=fn_name)['Policy']
    statements = json.loads(policy)['Statement'] 
    new_policy = None
    print(len(statements))
    for statement in statements:
        sid = statement['Sid']
        if statement['Principal']['Service'] == 'apigateway.amazonaws.com':
            print(statement['Sid'])
            if new_policy is None:
                new_policy = clone_policy(statement)
            client.remove_permission(FunctionName=fn_name, StatementId=sid)
    res = client.add_permission(
       FunctionName=fn_name,
       StatementId=new_policy['Sid'],
       Action=new_policy['Action'],
       Principal=new_policy['Principal']['Service'],
       SourceArn=new_policy['Condition']['ArnLike']['AWS:SourceArn']
    )
    print(res)
    # print('\n', new_policy)
    # sid_list = [item['Sid'] for item in statements][:-1]
    # for sid in sid_list:       
       # print("Removing policy SID {}".format(sid))
       #  client.remove_permission(FunctionName=fn_name, StatementId=sid)
    # with open('policies.json', 'wt') as fout:
    #     json.dump(statements, fout, indent=2)

if __name__ == '__main__':
    args = sys.argv
    if len(args) > 1:
        clean_policy(args[1])
    else:
        print('check_policy.py <function-name>')
